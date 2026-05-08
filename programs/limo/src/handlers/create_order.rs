use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use solana_program::{program::invoke, system_instruction};

use crate::{
    operations, seeds,
    state::{GlobalConfig, Order},
    token_operations::transfer_from_user_to_token_account,
    utils::constraints::token_2022::validate_token_extensions,
    utils::consts::FULL_BPS,
    LimoError, OrderDisplay, OrderType,
};

pub fn handler_create_order(
    ctx: Context<CreateOrder>,
    input_amount: u64,
    output_amount: u64,
    order_type: u8,
    tp_output_amount: u64,
    sl_output_amount: u64,
    active_duration_seconds: u64,
) -> Result<()> {
    let args = CreateOrderArgs {
        input_amount,
        output_amount,
        order_type,
        tp_output_amount,
        sl_output_amount,
        active_duration_seconds,
    };

    validate_token_extensions(
        &ctx.accounts.input_mint.to_account_info(),
        vec![&ctx.accounts.maker_ata.to_account_info()],
    )?;
    validate_token_extensions(&ctx.accounts.output_mint.to_account_info(), vec![])?;

    let (parsed_order_type, fees) = validate_order_args_and_calculate_fees(&ctx, args)?;

    let order = &mut ctx.accounts.order.load_init()?;
    let clock = Clock::get()?;
    initialize_parent_order(&ctx, order, args, clock.unix_timestamp)?;
    initialize_child_orders(&ctx, order, parsed_order_type, args, clock.unix_timestamp)?;
    fund_order_accounts(&ctx, args.input_amount, fees)?;

    msg!(
        "Created order {}, input_amount {}, input_mint {}, output_amount {}, output_mint {}",
        ctx.accounts.order.key(),
        input_amount,
        ctx.accounts.input_mint.key(),
        output_amount,
        ctx.accounts.output_mint.key(),
    );

    emit_order_display(&ctx, order)?;

    Ok(())
}

fn emit_order_display(ctx: &Context<CreateOrder>, order: &Order) -> Result<()> {
    emit_cpi!(OrderDisplay {
        initial_input_amount: order.initial_input_amount,
        expected_output_amount: order.expected_output_amount,
        remaining_input_amount: order.remaining_input_amount,
        filled_output_amount: order.filled_output_amount,
        tip_amount: order.tip_amount,
        number_of_fills: order.number_of_fills,
        on_event_output_amount_filled: 0,
        on_event_tip_amount: 0,
        order_type: order.order_type,
        status: order.status,
        last_updated_timestamp: order.last_updated_timestamp,
    });

    Ok(())
}

#[derive(Clone, Copy)]
struct CreateOrderArgs {
    input_amount: u64,
    output_amount: u64,
    order_type: u8,
    tp_output_amount: u64,
    sl_output_amount: u64,
    active_duration_seconds: u64,
}

#[derive(Clone, Copy)]
struct CreateOrderFees {
    create_order_fee: u64,
    lamports: u64,
}

fn validate_order_args_and_calculate_fees(
    ctx: &Context<CreateOrder>,
    args: CreateOrderArgs,
) -> Result<(OrderType, CreateOrderFees)> {
    require!(args.input_amount > 0, LimoError::OrderInputAmountInvalid);
    require!(args.output_amount > 0, LimoError::OrderOutputAmountInvalid);
    require!(
        ctx.accounts.input_mint.key() != ctx.accounts.output_mint.key(),
        LimoError::OrderSameMint
    );

    let parsed_order_type =
        OrderType::try_from(args.order_type).map_err(|_| LimoError::OrderTypeInvalid)?;
    require!(
        parsed_order_type != OrderType::LimitTP && parsed_order_type != OrderType::LimitSL,
        LimoError::OrderTypeInvalid
    );

    let gc_state = ctx.accounts.global_config.load()?;
    validate_tp_sl_args(args, parsed_order_type, &gc_state)?;

    let fees = CreateOrderFees {
        create_order_fee: operations::calculate_fee_amount(
            args.input_amount,
            gc_state.create_order_fee_bps,
        )?,
        lamports: gc_state.ata_creation_cost + gc_state.txn_fee_cost,
    };

    Ok((parsed_order_type, fees))
}

fn validate_tp_sl_args(
    args: CreateOrderArgs,
    parsed_order_type: OrderType,
    global_config: &GlobalConfig,
) -> Result<()> {
    if parsed_order_type == OrderType::Vanilla {
        require!(
            args.tp_output_amount == 0 && args.sl_output_amount == 0,
            LimoError::OrderParametersInvalid
        );
        return Ok(());
    }

    require!(global_config.tp_sl_enabled == 1, LimoError::TPSLNotEnabled);
    require!(
        args.tp_output_amount > 0 || args.sl_output_amount > 0,
        LimoError::OrderParametersInvalid
    );

    let tp_sl_min_distance = args
        .input_amount
        .checked_mul(u64::from(global_config.tp_sl_min_distance_bps))
        .unwrap()
        .checked_div(FULL_BPS)
        .unwrap_or(0);

    if args.tp_output_amount > 0 {
        require!(
            args.tp_output_amount >= args.input_amount.checked_add(tp_sl_min_distance).unwrap(),
            LimoError::TPSLMinDistanceNotMet
        );
    }
    if args.sl_output_amount > 0 {
        require!(
            args.sl_output_amount <= args.input_amount.checked_sub(tp_sl_min_distance).unwrap(),
            LimoError::TPSLMinDistanceNotMet
        );
    }

    Ok(())
}

fn initialize_parent_order(
    ctx: &Context<CreateOrder>,
    order: &mut Order,
    args: CreateOrderArgs,
    current_timestamp: i64,
) -> Result<()> {
    operations::create_order(
        order,
        ctx.accounts.global_config.key(),
        ctx.accounts.maker.key(),
        args.input_amount,
        args.output_amount,
        Pubkey::default(),
        ctx.accounts.input_mint.key(),
        ctx.accounts.output_mint.key(),
        ctx.accounts.input_token_program.key(),
        ctx.accounts.output_token_program.key(),
        args.order_type,
        ctx.bumps.input_vault,
        current_timestamp,
        args.active_duration_seconds,
    )
}

fn initialize_child_orders(
    ctx: &Context<CreateOrder>,
    order: &mut Order,
    parsed_order_type: OrderType,
    args: CreateOrderArgs,
    current_timestamp: i64,
) -> Result<()> {
    if parsed_order_type != OrderType::LimitParent {
        return Ok(());
    }

    require!(
        args.tp_output_amount > 0 || args.sl_output_amount > 0,
        LimoError::OrderParametersInvalid
    );
    ctx.accounts
        .output_vault
        .as_ref()
        .ok_or(LimoError::InvalidAccount)?;

    if args.tp_output_amount > 0 {
        let tp_order_account = ctx
            .accounts
            .tp_order
            .as_ref()
            .ok_or(LimoError::InvalidAccount)?;
        order.tp_child_order = initialize_child_order(
            ctx,
            ChildOrderSpec {
                account: tp_order_account,
                expected_output_amount: args.tp_output_amount,
                order_type: OrderType::LimitTP,
            },
            args,
            current_timestamp,
        )?;
    }
    if args.sl_output_amount > 0 {
        let sl_order_account = ctx
            .accounts
            .sl_order
            .as_ref()
            .ok_or(LimoError::InvalidAccount)?;
        order.sl_child_order = initialize_child_order(
            ctx,
            ChildOrderSpec {
                account: sl_order_account,
                expected_output_amount: args.sl_output_amount,
                order_type: OrderType::LimitSL,
            },
            args,
            current_timestamp,
        )?;
    }

    Ok(())
}

struct ChildOrderSpec<'a, 'info> {
    account: &'a AccountLoader<'info, Order>,
    expected_output_amount: u64,
    order_type: OrderType,
}

fn initialize_child_order(
    ctx: &Context<CreateOrder>,
    spec: ChildOrderSpec,
    args: CreateOrderArgs,
    current_timestamp: i64,
) -> Result<Pubkey> {
    let child_order = &mut spec.account.load_init()?;
    operations::create_order(
        child_order,
        ctx.accounts.global_config.key(),
        ctx.accounts.maker.key(),
        args.output_amount,
        spec.expected_output_amount,
        ctx.accounts.order.key(),
        ctx.accounts.output_mint.key(),
        ctx.accounts.input_mint.key(),
        ctx.accounts.output_token_program.key(),
        ctx.accounts.input_token_program.key(),
        spec.order_type as u8,
        ctx.bumps.output_vault,
        current_timestamp,
        args.active_duration_seconds,
    )?;

    Ok(spec.account.key())
}

fn fund_order_accounts(
    ctx: &Context<CreateOrder>,
    input_amount: u64,
    fees: CreateOrderFees,
) -> Result<()> {
    transfer_from_user_to_token_account(
        ctx.accounts.maker_ata.to_account_info(),
        ctx.accounts.input_vault.to_account_info(),
        ctx.accounts.maker.to_account_info(),
        ctx.accounts.input_mint.to_account_info(),
        ctx.accounts.input_token_program.to_account_info(),
        input_amount,
        ctx.accounts.input_mint.decimals,
    )?;

    if fees.create_order_fee > 0 {
        transfer_from_user_to_token_account(
            ctx.accounts.maker_ata.to_account_info(),
            ctx.accounts.input_fee_vault.to_account_info(),
            ctx.accounts.maker.to_account_info(),
            ctx.accounts.input_mint.to_account_info(),
            ctx.accounts.input_token_program.to_account_info(),
            fees.create_order_fee,
            ctx.accounts.input_mint.decimals,
        )?;
    }
    if fees.lamports > 0 {
        let maker = ctx.accounts.maker.key();
        let gc = ctx.accounts.global_config.key();
        let ixn = system_instruction::transfer(&maker, &gc, fees.lamports);

        invoke(
            &ixn,
            &[
                ctx.accounts.maker.to_account_info().clone(),
                ctx.accounts.global_config.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;
    }

    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct CreateOrder<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut, has_one = pda_authority)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account()]
    /// CHECK: pda_authority is a valid account
    pub pda_authority: AccountInfo<'info>,

    #[account(zero)]
    pub order: AccountLoader<'info, Order>,

    #[account(zero)]
    pub tp_order: Option<AccountLoader<'info, Order>>,

    #[account(zero)]
    pub sl_order: Option<AccountLoader<'info, Order>>,

    #[account(
        mint::token_program = input_token_program,
    )]
    pub input_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mint::token_program = output_token_program,
    )]
    pub output_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut,
        token::mint = input_mint,
        token::authority = maker
    )]
    pub maker_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::ESCROW_VAULT, global_config.key().as_ref(), input_mint.key().as_ref()],
        bump,
        token::mint = input_mint,
        token::authority = pda_authority
    )]
    pub input_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::FEE_VAULT, global_config.key().as_ref(), input_mint.key().as_ref()],
        bump,
        token::mint = input_mint,
        token::authority = pda_authority
    )]
    pub input_fee_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::ESCROW_VAULT, global_config.key().as_ref(), output_mint.key().as_ref()],
        bump,
        token::mint = output_mint,
        token::authority = pda_authority
    )]
    pub output_vault: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    pub input_token_program: Interface<'info, TokenInterface>,
    pub output_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
