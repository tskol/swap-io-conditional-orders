use anchor_lang::prelude::*;
use derivative::Derivative;

#[derive(PartialEq, Derivative, Default)]
#[derivative(Debug)]
#[account(zero_copy)]
pub struct Order {
    pub global_config: Pubkey,
    pub maker: Pubkey,

    pub input_mint: Pubkey,
    pub input_mint_program_id: Pubkey,
    pub output_mint: Pubkey,
    pub output_mint_program_id: Pubkey,

    pub parent_order: Pubkey,
    pub tp_child_order: Pubkey,
    pub sl_child_order: Pubkey,
    pub available_child_input_amount: u64,

    pub initial_input_amount: u64,
    pub expected_output_amount: u64,
    pub remaining_input_amount: u64,
    pub filled_output_amount: u64,
    pub tip_amount: u64,
    pub number_of_fills: u64,

    pub order_type: u8,
    pub status: u8,
    pub in_vault_bump: u8,
    pub flash_ix_lock: u8,

    pub permissionless: u8,

    pub padding0: [u8; 3],

    pub last_updated_timestamp: u64,

    pub flash_start_taker_output_balance: u64,

    pub counterparty: Pubkey,

    pub expiry_timestamp: u64,

    pub padding: [u64; 14],
}

#[event]
pub struct OrderDisplay {
    pub initial_input_amount: u64,
    pub expected_output_amount: u64,
    pub remaining_input_amount: u64,
    pub filled_output_amount: u64,
    pub tip_amount: u64,
    pub number_of_fills: u64,

    pub on_event_output_amount_filled: u64,
    pub on_event_tip_amount: u64,

    pub order_type: u8,
    pub status: u8,

    pub last_updated_timestamp: u64,
}

pub struct ExecuteOrderEffects {
    pub input_to_send_to_taker: u64,
    pub output_to_send_to_maker: u64,
    pub output_to_send_to_protocol: u64,
    pub output_keeper_fee: u64,
}

pub struct TipCalcs {
    pub host_tip: u64,
    pub maker_tip: u64,
}
