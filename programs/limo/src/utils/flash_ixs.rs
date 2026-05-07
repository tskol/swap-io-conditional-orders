use anchor_lang::{
    prelude::*,
    solana_program::{
        self,
        instruction::Instruction,
        sysvar::instructions::{load_current_index_checked, load_instruction_at_checked},
    },
    AnchorDeserialize, Discriminator,
};
use anchor_spl::{
    associated_token,
    token::spl_token,
    token_2022::{self, spl_token_2022::instruction::TokenInstruction},
};
use solana_program::pubkey;

use crate::LimoError;

const COMPUTE_BUDGET_PUBKEY: Pubkey = pubkey!("ComputeBudget111111111111111111111111111111");

pub fn ensure_second_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_second_ix_match_internal(&instruction_loader, input_mint, output_mint)
}

fn ensure_second_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let second_ix = search_second_ix(current_idx, instruction_loader, input_mint, output_mint)?;
    if let Some(discriminator) = second_ix.data.get(..8) {
        if discriminator != T::discriminator() {
            msg!("Extra ix is not the expected one");
            return err!(LimoError::FlashTxWithUnexpectedIxs);
        }
    } else {
        msg!("Extra ix has no valid discriminator");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    let current_ix = instruction_loader.load_instruction_at(current_idx)?;
    check_same_accounts(&current_ix, &second_ix)?;

    Ok(T::try_from_slice(&second_ix.data[8..])?)
}

fn search_second_ix(
    current_idx: usize,
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<Instruction> {
    for idx in 0..current_idx {
        let ix = instruction_loader.load_instruction_at(idx)?;

        require!(
            program_id_allowed(ix.program_id),
            LimoError::FlashTxWithUnexpectedIxs
        );

        if ix.program_id == token_2022::ID {
            token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
        }
    }

    let mut found_extra_ix = None;
    let mut ix_iterator =
        ix_utils::IxIterator::new_at(current_idx.checked_add(1).unwrap(), instruction_loader);

    for ix in ix_iterator.by_ref() {
        if let Err(error) = ix {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(error.into());
        }
        let ix = ix?;
        if ix.program_id == crate::id() {
            found_extra_ix = Some(ix);
            break;
        }
    }

    let extra_ix = found_extra_ix.ok_or_else(|| error!(LimoError::FlashIxsNotEnded))?;

    for ix in ix_iterator.by_ref() {
        if let Err(error) = ix {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(error.into());
        }
        let ix = ix?;
        require!(
            program_id_allowed(ix.program_id),
            LimoError::FlashTxWithUnexpectedIxs
        );
        if ix.program_id == token_2022::ID {
            token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
        }
    }

    Ok(extra_ix)
}

fn program_id_allowed(program_id: Pubkey) -> bool {
    program_id == COMPUTE_BUDGET_PUBKEY
        || program_id == spl_token::ID
        || program_id == token_2022::ID
        || program_id == associated_token::ID
}

pub fn ensure_first_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_first_ix_match_internal(&instruction_loader, input_mint, output_mint)
}

fn ensure_first_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let first_ix = search_first_ix(current_idx, instruction_loader, input_mint, output_mint)?;
    if let Some(discriminator) = first_ix.data.get(..8) {
        if discriminator != T::discriminator() {
            msg!("Extra ix is not the expected one");
            return err!(LimoError::FlashTxWithUnexpectedIxs);
        }
    } else {
        msg!("Extra ix has no valid discriminator");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    let current_ix = instruction_loader.load_instruction_at(current_idx)?;
    check_same_accounts(&first_ix, &current_ix)?;

    Ok(T::try_from_slice(&first_ix.data[8..])?)
}

fn search_first_ix(
    current_idx: usize,
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<Instruction> {
    let mut ix_iterator =
        ix_utils::IxIterator::new_at(current_idx.checked_add(1).unwrap(), instruction_loader);

    for ix in ix_iterator.by_ref() {
        if let Err(error) = ix {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(error.into());
        }
        let ix = ix?;
        require!(
            program_id_allowed(ix.program_id),
            LimoError::FlashTxWithUnexpectedIxs
        );
        if ix.program_id == token_2022::ID {
            token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
        }
    }

    let mut found_extra_ix = None;

    for idx in 0..current_idx {
        let ix = instruction_loader.load_instruction_at(idx)?;
        if ix.program_id == crate::id() {
            found_extra_ix = Some(ix);
            break;
        } else {
            require!(
                program_id_allowed(ix.program_id),
                LimoError::FlashTxWithUnexpectedIxs
            );
            if ix.program_id == token_2022::ID {
                token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
            }
        }
    }

    let extra_ix = found_extra_ix.ok_or_else(|| error!(LimoError::FlashIxsNotStarted))?;

    Ok(extra_ix)
}

fn token_2022_verify_ix_and_mints(
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

pub fn check_same_accounts(start_ix: &Instruction, end_ix: &Instruction) -> Result<()> {
    if end_ix.accounts.len() != start_ix.accounts.len() {
        msg!("Number of accounts mismatch between start and end ix");
        return err!(LimoError::FlashIxsAccountMismatch);
    }

    for (idx, (account_start, account_end)) in start_ix
        .accounts
        .iter()
        .zip(end_ix.accounts.iter())
        .enumerate()
    {
        let account_start_pk = &account_start.pubkey;
        let account_end_pk = &account_end.pubkey;
        if account_start_pk != account_end_pk {
            msg!("Some accounts in assert_user_swap_balances tx differ. index: {idx}, start:{account_start_pk}, end:{account_end_pk}",);
            return err!(LimoError::FlashIxsAccountMismatch);
        }
    }
    Ok(())
}

pub mod ix_utils {
    use super::*;

    pub trait InstructionLoader {
        fn load_instruction_at(
            &self,
            index: usize,
        ) -> std::result::Result<Instruction, ProgramError>;
        fn load_current_index(&self) -> std::result::Result<u16, ProgramError>;
    }

    pub struct BpfInstructionLoader<'a, 'info> {
        pub instruction_sysvar_account_info: &'a AccountInfo<'info>,
    }

    impl<'a, 'info> InstructionLoader for BpfInstructionLoader<'a, 'info> {
        fn load_instruction_at(
            &self,
            index: usize,
        ) -> std::result::Result<Instruction, ProgramError> {
            load_instruction_at_checked(index, self.instruction_sysvar_account_info)
        }

        fn load_current_index(&self) -> std::result::Result<u16, ProgramError> {
            load_current_index_checked(self.instruction_sysvar_account_info)
        }
    }

    pub struct IxIterator<'a, IxLoader: InstructionLoader> {
        current_ix: usize,
        instruction_loader: &'a IxLoader,
    }

    impl<'a, IxLoader> IxIterator<'a, IxLoader>
    where
        IxLoader: InstructionLoader,
    {
        pub fn new_at(start_ix_index: usize, instruction_loader: &'a IxLoader) -> Self {
            Self {
                current_ix: start_ix_index,
                instruction_loader,
            }
        }
    }

    impl<IxLoader> Iterator for IxIterator<'_, IxLoader>
    where
        IxLoader: InstructionLoader,
    {
        type Item = std::result::Result<Instruction, ProgramError>;

        fn next(&mut self) -> Option<Self::Item> {
            match self.instruction_loader.load_instruction_at(self.current_ix) {
                Ok(ix) => {
                    self.current_ix = self.current_ix.checked_add(1).unwrap();
                    Some(Ok(ix))
                }
                Err(ProgramError::InvalidArgument) => None,
                Err(e) => Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::{instruction::AccountMeta, program_error::ProgramError};
    use anchor_lang::AnchorSerialize;

    const TEST_ARGS_DISCRIMINATOR: [u8; 8] = [9; 8];

    #[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq)]
    struct TestArgs {
        value: u64,
    }

    impl Discriminator for TestArgs {
        const DISCRIMINATOR: [u8; 8] = TEST_ARGS_DISCRIMINATOR;
    }

    struct FakeInstructionLoader {
        instructions: Vec<Instruction>,
        current_index: u16,
    }

    impl ix_utils::InstructionLoader for FakeInstructionLoader {
        fn load_instruction_at(
            &self,
            index: usize,
        ) -> std::result::Result<Instruction, ProgramError> {
            self.instructions
                .get(index)
                .cloned()
                .ok_or(ProgramError::InvalidArgument)
        }

        fn load_current_index(&self) -> std::result::Result<u16, ProgramError> {
            Ok(self.current_index)
        }
    }

    fn shared_accounts() -> Vec<AccountMeta> {
        vec![AccountMeta::new_readonly(Pubkey::new_unique(), false)]
    }

    fn instruction_with_program(
        program_id: Pubkey,
        data: Vec<u8>,
        accounts: Vec<AccountMeta>,
    ) -> Instruction {
        Instruction {
            program_id,
            accounts,
            data,
        }
    }

    fn limo_instruction(data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
        instruction_with_program(crate::id(), data, accounts)
    }

    fn allowed_instruction() -> Instruction {
        instruction_with_program(COMPUTE_BUDGET_PUBKEY, vec![], vec![])
    }

    fn disallowed_instruction() -> Instruction {
        instruction_with_program(Pubkey::new_unique(), vec![], vec![])
    }

    fn test_args_data(value: u64) -> Vec<u8> {
        let mut data = TestArgs::discriminator().to_vec();
        TestArgs { value }.serialize(&mut data).unwrap();
        data
    }

    fn token_2022_instruction(ix: TokenInstruction, accounts: Vec<Pubkey>) -> Instruction {
        Instruction {
            program_id: token_2022::ID,
            accounts: accounts
                .into_iter()
                .map(|pubkey| AccountMeta::new_readonly(pubkey, false))
                .collect(),
            data: ix.pack(),
        }
    }

    #[test]
    fn ensure_first_ix_match_internal_deserializes_matching_first_ix() {
        let input_mint = Pubkey::new_unique();
        let accounts = shared_accounts();
        let loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(test_args_data(7), accounts.clone()),
                allowed_instruction(),
                limo_instruction(vec![1; 8], accounts),
                token_2022_instruction(
                    TokenInstruction::TransferChecked {
                        amount: 1,
                        decimals: 6,
                    },
                    vec![Pubkey::new_unique(), input_mint],
                ),
            ],
            current_index: 2,
        };

        let args = ensure_first_ix_match_internal::<TestArgs>(
            &loader,
            &input_mint,
            &Pubkey::new_unique(),
        )
        .unwrap();

        assert_eq!(args, TestArgs { value: 7 });
    }

    #[test]
    fn ensure_second_ix_match_internal_deserializes_matching_second_ix() {
        let input_mint = Pubkey::new_unique();
        let accounts = shared_accounts();
        let loader = FakeInstructionLoader {
            instructions: vec![
                token_2022_instruction(
                    TokenInstruction::TransferChecked {
                        amount: 1,
                        decimals: 6,
                    },
                    vec![Pubkey::new_unique(), input_mint],
                ),
                limo_instruction(vec![1; 8], accounts.clone()),
                limo_instruction(test_args_data(42), accounts),
                allowed_instruction(),
            ],
            current_index: 1,
        };

        let args = ensure_second_ix_match_internal::<TestArgs>(
            &loader,
            &input_mint,
            &Pubkey::new_unique(),
        )
        .unwrap();

        assert_eq!(args, TestArgs { value: 42 });
    }

    #[test]
    fn search_flash_ixs_reject_unexpected_program_ids() {
        let accounts = shared_accounts();
        let first_after_current_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(test_args_data(1), accounts.clone()),
                limo_instruction(vec![1; 8], accounts),
                disallowed_instruction(),
            ],
            current_index: 1,
        };
        let first_before_current_loader = FakeInstructionLoader {
            instructions: vec![
                disallowed_instruction(),
                limo_instruction(vec![1; 8], shared_accounts()),
            ],
            current_index: 1,
        };
        let second_before_current_loader = FakeInstructionLoader {
            instructions: vec![
                disallowed_instruction(),
                limo_instruction(vec![1; 8], shared_accounts()),
                limo_instruction(test_args_data(1), shared_accounts()),
            ],
            current_index: 1,
        };
        let second_after_extra_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(vec![1; 8], shared_accounts()),
                limo_instruction(test_args_data(1), shared_accounts()),
                disallowed_instruction(),
            ],
            current_index: 0,
        };

        assert!(search_first_ix(
            1,
            &first_after_current_loader,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
        assert!(search_first_ix(
            1,
            &first_before_current_loader,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
        assert!(search_second_ix(
            1,
            &second_before_current_loader,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
        assert!(search_second_ix(
            0,
            &second_after_extra_loader,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
    }

    #[test]
    fn ensure_flash_ixs_reject_bad_discriminator_or_accounts() {
        let accounts = shared_accounts();
        let wrong_discriminator_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(vec![8; 8], accounts.clone()),
                limo_instruction(vec![1; 8], accounts),
            ],
            current_index: 1,
        };
        let short_discriminator_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(vec![1; 4], shared_accounts()),
                limo_instruction(vec![1; 8], shared_accounts()),
            ],
            current_index: 1,
        };
        let account_mismatch_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(test_args_data(5), shared_accounts()),
                limo_instruction(vec![1; 8], shared_accounts()),
            ],
            current_index: 1,
        };

        assert!(ensure_first_ix_match_internal::<TestArgs>(
            &wrong_discriminator_loader,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
        assert!(ensure_first_ix_match_internal::<TestArgs>(
            &short_discriminator_loader,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
        assert!(ensure_first_ix_match_internal::<TestArgs>(
            &account_mismatch_loader,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
    }

    #[test]
    fn token_2022_verify_ix_and_mints_rejects_missing_mint_account() {
        let instruction = token_2022_instruction(
            TokenInstruction::TransferChecked {
                amount: 1,
                decimals: 6,
            },
            vec![],
        );

        assert!(token_2022_verify_ix_and_mints(
            &instruction,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
    }

    #[test]
    fn token_2022_verify_ix_and_mints_accepts_non_token_2022_program() {
        let instruction = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![],
        };

        token_2022_verify_ix_and_mints(&instruction, &Pubkey::new_unique(), &Pubkey::new_unique())
            .unwrap();
    }

    #[test]
    fn token_2022_verify_ix_and_mints_accepts_permitted_instruction_without_mint() {
        let instruction = token_2022_instruction(TokenInstruction::Approve { amount: 1 }, vec![]);

        token_2022_verify_ix_and_mints(&instruction, &Pubkey::new_unique(), &Pubkey::new_unique())
            .unwrap();
    }

    #[test]
    fn token_2022_verify_ix_and_mints_accepts_matching_mint() {
        let input_mint = Pubkey::new_unique();
        let instruction = token_2022_instruction(
            TokenInstruction::TransferChecked {
                amount: 1,
                decimals: 6,
            },
            vec![Pubkey::new_unique(), input_mint],
        );

        token_2022_verify_ix_and_mints(&instruction, &input_mint, &Pubkey::new_unique()).unwrap();
    }

    #[test]
    fn token_2022_verify_ix_and_mints_rejects_wrong_mint_or_bad_data() {
        let wrong_mint_ix = token_2022_instruction(
            TokenInstruction::TransferChecked {
                amount: 1,
                decimals: 6,
            },
            vec![Pubkey::new_unique(), Pubkey::new_unique()],
        );
        let bad_data_ix = Instruction {
            program_id: token_2022::ID,
            accounts: vec![],
            data: vec![255],
        };

        assert!(token_2022_verify_ix_and_mints(
            &wrong_mint_ix,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
        assert!(token_2022_verify_ix_and_mints(
            &bad_data_ix,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        )
        .is_err());
    }
}
