use super::*;
use crate::utils::ix_test_helpers::{
    instruction_with_accounts as instruction_with_program,
    limo_instruction_with_accounts as limo_instruction, shared_accounts, test_args_data,
    FakeInstructionLoader, TestArgs,
};
use anchor_lang::solana_program::instruction::AccountMeta;

fn allowed_instruction() -> Instruction {
    instruction_with_program(COMPUTE_BUDGET_PUBKEY, vec![], vec![])
}

fn disallowed_instruction() -> Instruction {
    instruction_with_program(Pubkey::new_unique(), vec![], vec![])
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

    let args =
        ensure_first_ix_match_internal::<TestArgs>(&loader, &input_mint, &Pubkey::new_unique())
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

    let args =
        ensure_second_ix_match_internal::<TestArgs>(&loader, &input_mint, &Pubkey::new_unique())
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
