use super::*;
use crate::utils::ix_test_helpers::{
    instruction, ordo_instruction_with_accounts as ordo_instruction, shared_accounts,
    test_args_data, FakeInstructionLoader, TestArgs,
};

#[test]
fn ensure_start_ix_match_internal_deserializes_matching_start_ix() {
    let swap_program_id = Pubkey::new_unique();
    let accounts = shared_accounts();
    let loader = FakeInstructionLoader {
        instructions: vec![
            ordo_instruction(test_args_data(7), accounts.clone()),
            instruction(swap_program_id),
            ordo_instruction(vec![1; 8], accounts),
        ],
        current_index: 2,
    };

    let args = ensure_start_ix_match_internal::<TestArgs>(&loader, &swap_program_id).unwrap();

    assert_eq!(args, TestArgs { value: 7 });
}

#[test]
fn ensure_end_ix_match_internal_deserializes_matching_end_ix() {
    let swap_program_id = Pubkey::new_unique();
    let accounts = shared_accounts();
    let loader = FakeInstructionLoader {
        instructions: vec![
            ordo_instruction(vec![1; 8], accounts.clone()),
            instruction(swap_program_id),
            ordo_instruction(test_args_data(42), accounts),
        ],
        current_index: 0,
    };

    let args = ensure_end_ix_match_internal::<TestArgs>(&loader, &swap_program_id).unwrap();

    assert_eq!(args, TestArgs { value: 42 });
}

#[test]
fn search_start_ix_rejects_unexpected_instruction_between_start_and_end() {
    let swap_program_id = Pubkey::new_unique();
    let loader = FakeInstructionLoader {
        instructions: vec![
            instruction(crate::id()),
            instruction(Pubkey::new_unique()),
            instruction(swap_program_id),
            instruction(crate::id()),
        ],
        current_index: 3,
    };

    assert!(search_start_ix(3, &loader, &swap_program_id).is_err());
}

#[test]
fn search_start_ix_accepts_single_swap_between_start_and_end() {
    let swap_program_id = Pubkey::new_unique();
    let loader = FakeInstructionLoader {
        instructions: vec![
            instruction(crate::id()),
            instruction(swap_program_id),
            instruction(crate::id()),
        ],
        current_index: 2,
    };

    let start_ix = search_start_ix(2, &loader, &swap_program_id).unwrap();

    assert_eq!(start_ix.program_id, crate::id());
}

#[test]
fn search_end_ix_accepts_single_swap_between_start_and_end() {
    let swap_program_id = Pubkey::new_unique();
    let loader = FakeInstructionLoader {
        instructions: vec![
            instruction(crate::id()),
            instruction(swap_program_id),
            instruction(crate::id()),
        ],
        current_index: 0,
    };

    let end_ix = search_end_ix(0, &loader, &swap_program_id).unwrap();

    assert_eq!(end_ix.program_id, crate::id());
}

#[test]
fn search_start_ix_rejects_missing_or_repeated_swap() {
    let swap_program_id = Pubkey::new_unique();
    let missing_swap_loader = FakeInstructionLoader {
        instructions: vec![instruction(crate::id()), instruction(crate::id())],
        current_index: 1,
    };
    let repeated_swap_loader = FakeInstructionLoader {
        instructions: vec![
            instruction(crate::id()),
            instruction(swap_program_id),
            instruction(swap_program_id),
            instruction(crate::id()),
        ],
        current_index: 3,
    };

    assert!(search_start_ix(1, &missing_swap_loader, &swap_program_id).is_err());
    assert!(search_start_ix(3, &repeated_swap_loader, &swap_program_id).is_err());
}

#[test]
fn search_end_ix_rejects_unexpected_instruction_between_start_and_end() {
    let swap_program_id = Pubkey::new_unique();
    let loader = FakeInstructionLoader {
        instructions: vec![
            instruction(crate::id()),
            instruction(swap_program_id),
            instruction(Pubkey::new_unique()),
            instruction(crate::id()),
        ],
        current_index: 0,
    };

    assert!(search_end_ix(0, &loader, &swap_program_id).is_err());
}

#[test]
fn ensure_log_ixs_reject_bad_discriminator_or_accounts() {
    let swap_program_id = Pubkey::new_unique();
    let accounts = shared_accounts();
    let wrong_discriminator_loader = FakeInstructionLoader {
        instructions: vec![
            ordo_instruction(vec![8; 8], accounts.clone()),
            instruction(swap_program_id),
            ordo_instruction(vec![1; 8], accounts),
        ],
        current_index: 2,
    };
    let short_discriminator_loader = FakeInstructionLoader {
        instructions: vec![
            ordo_instruction(vec![1; 4], shared_accounts()),
            instruction(swap_program_id),
            ordo_instruction(vec![1; 8], shared_accounts()),
        ],
        current_index: 2,
    };
    let account_mismatch_loader = FakeInstructionLoader {
        instructions: vec![
            ordo_instruction(test_args_data(5), shared_accounts()),
            instruction(swap_program_id),
            ordo_instruction(vec![1; 8], shared_accounts()),
        ],
        current_index: 2,
    };

    assert!(ensure_start_ix_match_internal::<TestArgs>(
        &wrong_discriminator_loader,
        &swap_program_id,
    )
    .is_err());
    assert!(ensure_start_ix_match_internal::<TestArgs>(
        &short_discriminator_loader,
        &swap_program_id,
    )
    .is_err());
    assert!(
        ensure_start_ix_match_internal::<TestArgs>(&account_mismatch_loader, &swap_program_id,)
            .is_err()
    );
}

#[test]
fn search_end_ix_rejects_missing_or_repeated_swap() {
    let swap_program_id = Pubkey::new_unique();
    let missing_swap_loader = FakeInstructionLoader {
        instructions: vec![instruction(crate::id()), instruction(crate::id())],
        current_index: 0,
    };
    let repeated_swap_loader = FakeInstructionLoader {
        instructions: vec![
            instruction(crate::id()),
            instruction(swap_program_id),
            instruction(swap_program_id),
            instruction(crate::id()),
        ],
        current_index: 0,
    };

    assert!(search_end_ix(0, &missing_swap_loader, &swap_program_id).is_err());
    assert!(search_end_ix(0, &repeated_swap_loader, &swap_program_id).is_err());
}
