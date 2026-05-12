use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::TokenAccount;
use solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId};

use crate::{seeds, utils::consts::USER_SWAP_BALANCE_STATE_SIZE, UserSwapBalancesState};

#[derive(Accounts)]
pub struct AssertUserSwapBalancesStartContext<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        token::authority = maker
    )]
    pub input_ta: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        token::authority = maker
    )]
    pub output_ta: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        seeds = [seeds::ASSERT_SWAP_BALANCES_SEED, maker.key().as_ref()],
        bump,
        payer = maker,
        space = USER_SWAP_BALANCE_STATE_SIZE + 8
    )]
    pub user_swap_balance_state: AccountLoader<'info, UserSwapBalancesState>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,

    /// CHECK: SysInstructions is a valid sysvar
    #[account(address = SysInstructions::id())]
    pub sysvar_instructions: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct AssertUserSwapBalancesEndContext<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        token::authority = maker
    )]
    pub input_ta: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        token::authority = maker
    )]
    pub output_ta: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::ASSERT_SWAP_BALANCES_SEED, maker.key().as_ref()],
        bump,
        close = maker,
    )]
    pub user_swap_balance_state: AccountLoader<'info, UserSwapBalancesState>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,

    /// CHECK: SysInstructions is a valid sysvar
    #[account(address = SysInstructions::id())]
    pub sysvar_instructions: AccountInfo<'info>,
}
