use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    global_seeds,
    seeds::{FEE_VAULT, GLOBAL_AUTH},
    token_operations::transfer_from_vault_to_token_account,
    GlobalConfig, LimoError,
};

pub fn withdraw_fee(ctx: Context<WithdrawFee>, amount: u64) -> Result<()> {
    require!(amount > 0, LimoError::InvalidWithdrawFeeAmount);
    let global_config = &mut ctx.accounts.global_config.load_mut()?;

    let gc = ctx.accounts.global_config.key();
    let seeds: &[&[u8]] = global_seeds!(global_config.pda_authority_bump as u8, &gc);

    transfer_from_vault_to_token_account(
        ctx.accounts.fee_receiver_ata.to_account_info(),
        ctx.accounts.fee_vault.to_account_info(),
        ctx.accounts.pda_authority.to_account_info(),
        ctx.accounts.token_mint.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        seeds,
        amount,
        ctx.accounts.token_mint.decimals,
    )
    .unwrap();

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawFee<'info> {
    #[account(mut)]
    pub admin_authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: fee_receiver is a valid account
    pub fee_receiver: AccountInfo<'info>,

    #[account(mut,
        token::mint = token_mint,
        token::authority = fee_receiver
    )]
    pub fee_receiver_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        has_one = pda_authority,
        has_one = admin_authority
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(mut)]
    /// CHECK: pda_authority is a valid account
    pub pda_authority: AccountInfo<'info>,

    #[account(
        mint::token_program = token_program,
    )]
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut,
        seeds = [FEE_VAULT, global_config.key().as_ref(), token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = pda_authority
    )]
    pub fee_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
