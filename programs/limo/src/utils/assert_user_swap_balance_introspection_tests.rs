use super::*;
use crate::utils::ix_test_helpers::{
    instruction_with_accounts, limo_instruction, limo_instruction_with_accounts, shared_accounts,
    test_args_data, FakeInstructionLoader, TestArgs,
};

const START_DISCRIMINATOR: [u8; 8] = [1; 8];
const END_DISCRIMINATOR: [u8; 8] = [2; 8];

#[test]
fn ensure_end_ix_match_internal_deserializes_matching_end_ix() {
    let accounts = shared_accounts();
    let loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction_with_accounts(START_DISCRIMINATOR.to_vec(), accounts.clone()),
            instruction_with_accounts(Pubkey::new_unique(), vec![3; 8], shared_accounts()),
            limo_instruction_with_accounts(test_args_data(42), accounts),
        ],
        current_index: 0,
    };

    let args = ensure_end_ix_match_internal::<TestArgs>(&loader, &START_DISCRIMINATOR).unwrap();

    assert_eq!(args, TestArgs { value: 42 });
}

#[test]
fn ensure_start_ix_match_internal_deserializes_matching_start_ix() {
    let accounts = shared_accounts();
    let loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction_with_accounts(test_args_data(7), accounts.clone()),
            instruction_with_accounts(Pubkey::new_unique(), vec![3; 8], shared_accounts()),
            limo_instruction_with_accounts(END_DISCRIMINATOR.to_vec(), accounts),
        ],
        current_index: 2,
    };

    let args = ensure_start_ix_match_internal::<TestArgs>(&loader, &END_DISCRIMINATOR).unwrap();

    assert_eq!(args, TestArgs { value: 7 });
}

#[test]
fn search_end_ix_rejects_short_limo_discriminator() {
    let loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction(START_DISCRIMINATOR.to_vec()),
            limo_instruction(vec![7; 4]),
        ],
        current_index: 0,
    };

    assert!(search_end_ix(0, &loader, &START_DISCRIMINATOR, &END_DISCRIMINATOR,).is_err());
}

#[test]
fn search_end_ix_rejects_repeated_end_or_start_ix() {
    let repeated_end_loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction(START_DISCRIMINATOR.to_vec()),
            limo_instruction(END_DISCRIMINATOR.to_vec()),
            limo_instruction(END_DISCRIMINATOR.to_vec()),
        ],
        current_index: 0,
    };
    let repeated_start_loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction(START_DISCRIMINATOR.to_vec()),
            limo_instruction(START_DISCRIMINATOR.to_vec()),
        ],
        current_index: 0,
    };

    assert!(search_end_ix(
        0,
        &repeated_end_loader,
        &START_DISCRIMINATOR,
        &END_DISCRIMINATOR,
    )
    .is_err());
    assert!(search_end_ix(
        0,
        &repeated_start_loader,
        &START_DISCRIMINATOR,
        &END_DISCRIMINATOR,
    )
    .is_err());
}

#[test]
fn search_start_ix_rejects_short_limo_discriminator() {
    let loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction(vec![7; 4]),
            limo_instruction(END_DISCRIMINATOR.to_vec()),
        ],
        current_index: 1,
    };

    assert!(search_start_ix(1, &loader, &START_DISCRIMINATOR, &END_DISCRIMINATOR,).is_err());
}

#[test]
fn search_start_ix_rejects_repeated_start_or_end_ix() {
    let repeated_start_loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction(START_DISCRIMINATOR.to_vec()),
            limo_instruction(START_DISCRIMINATOR.to_vec()),
            limo_instruction(END_DISCRIMINATOR.to_vec()),
        ],
        current_index: 2,
    };
    let repeated_end_loader = FakeInstructionLoader {
        instructions: vec![
            limo_instruction(START_DISCRIMINATOR.to_vec()),
            limo_instruction(END_DISCRIMINATOR.to_vec()),
            limo_instruction(END_DISCRIMINATOR.to_vec()),
        ],
        current_index: 2,
    };

    assert!(search_start_ix(
        2,
        &repeated_start_loader,
        &START_DISCRIMINATOR,
        &END_DISCRIMINATOR,
    )
    .is_err());
    assert!(search_start_ix(
        2,
        &repeated_end_loader,
        &START_DISCRIMINATOR,
        &END_DISCRIMINATOR,
    )
    .is_err());
}
