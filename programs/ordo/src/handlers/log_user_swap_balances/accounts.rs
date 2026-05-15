use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::Mint;
use solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId};

use crate::{seeds, utils::consts::USER_SWAP_BALANCE_STATE_SIZE, UserSwapBalancesState};

#[derive(Accounts)]
pub struct LogUserSwapBalances<'info> {
    #[account()]
    pub maker: Signer<'info>,

    pub input_mint: Box<InterfaceAccount<'info, Mint>>,

    pub output_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: input_ta is a valid account
    pub input_ta: UncheckedAccount<'info>,

    /// CHECK: output_ta is a valid account
    pub output_ta: UncheckedAccount<'info>,

    pub pda_referrer: Option<AccountInfo<'info>>,

    /// CHECK: swap_program_id is a valid account
    pub swap_program_id: AccountInfo<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct LogUserSwapBalancesStartContext<'info> {
    pub base_accounts: LogUserSwapBalances<'info>,

    #[account(
        init,
        seeds = [seeds::USER_SWAP_BALANCES_SEED, base_accounts.maker.key().as_ref()],
        bump,
        payer = base_accounts.maker,
        space = USER_SWAP_BALANCE_STATE_SIZE + 8
    )]
    pub user_swap_balance_state: AccountLoader<'info, UserSwapBalancesState>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,

    #[account(address = SysInstructions::id())]
    /// CHECK: SysInstructions is a valid sysvar
    pub sysvar_instructions: AccountInfo<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct LogUserSwapBalancesEndContext<'info> {
    pub base_accounts: LogUserSwapBalances<'info>,

    #[account(mut,
        seeds = [seeds::USER_SWAP_BALANCES_SEED, base_accounts.maker.key().as_ref()],
        bump,
    )]
    pub user_swap_balance_state: AccountLoader<'info, UserSwapBalancesState>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,

    #[account(address = SysInstructions::id())]
    /// CHECK: SysInstructions is a valid sysvar
    pub sysvar_instructions: AccountInfo<'info>,
}
