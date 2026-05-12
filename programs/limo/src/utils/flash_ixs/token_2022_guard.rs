use super::*;

pub(super) fn token_2022_verify_ix_and_mints(
    instruction: &Instruction,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<()> {
    if instruction.program_id != token_2022::ID {
        return Ok(());
    }

    let ix = TokenInstruction::unpack(&instruction.data).map_err(|err| {
        msg!("Error unpacking token instruction: {:?}", err);
        err
    })?;

    let is_permitted = match ix {
        TokenInstruction::Approve { .. }
        | TokenInstruction::ApproveChecked { .. }
        | TokenInstruction::InitializeImmutableOwner
        | TokenInstruction::InitializeMultisig { .. }
        | TokenInstruction::InitializeMultisig2 { .. }
        | TokenInstruction::Revoke
        | TokenInstruction::SyncNative
        | TokenInstruction::CloseAccount => true,

        TokenInstruction::Burn { .. }
        | TokenInstruction::BurnChecked { .. }
        | TokenInstruction::FreezeAccount
        | TokenInstruction::GetAccountDataSize { .. }
        | TokenInstruction::InitializeAccount
        | TokenInstruction::InitializeAccount2 { .. }
        | TokenInstruction::InitializeAccount3 { .. }
        | TokenInstruction::InitializeMint { .. }
        | TokenInstruction::InitializeMint2 { .. }
        | TokenInstruction::MintTo { .. }
        | TokenInstruction::MintToChecked { .. }
        | TokenInstruction::TransferChecked { .. } => {
            let mint_index = token_2022_mint_account_index(&ix);
            let Some(mint_account) = instruction.accounts.get(mint_index) else {
                msg!("Token instruction missing mint account");
                return err!(LimoError::FlashTxWithUnexpectedIxs);
            };
            let mint = mint_account.pubkey;

            *input_mint == mint || *output_mint == mint
        }

        #[allow(deprecated)]
        TokenInstruction::SetAuthority { .. } | TokenInstruction::Transfer { .. } => false,

        _ => false,
    };

    require!(is_permitted, LimoError::FlashTxWithUnexpectedIxs);

    Ok(())
}

fn token_2022_mint_account_index(ix: &TokenInstruction) -> usize {
    match ix {
        TokenInstruction::GetAccountDataSize { .. }
        | TokenInstruction::InitializeMint { .. }
        | TokenInstruction::InitializeMint2 { .. }
        | TokenInstruction::MintTo { .. }
        | TokenInstruction::MintToChecked { .. } => 0,
        TokenInstruction::Burn { .. }
        | TokenInstruction::BurnChecked { .. }
        | TokenInstruction::FreezeAccount
        | TokenInstruction::InitializeAccount
        | TokenInstruction::InitializeAccount2 { .. }
        | TokenInstruction::InitializeAccount3 { .. }
        | TokenInstruction::TransferChecked { .. } => 1,
        _ => 0,
    }
}
