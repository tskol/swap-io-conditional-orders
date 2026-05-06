use anchor_lang::prelude::Pubkey;
use limo::{
    operations::create_order,
    state::{Order, OrderStatus, OrderType},
};

#[test]
fn create_order_initializes_vanilla_order_fields() {
    let mut order = Order::default();
    let global_config = Pubkey::new_unique();
    let maker = Pubkey::new_unique();
    let input_mint = Pubkey::new_unique();
    let output_mint = Pubkey::new_unique();
    let input_program = Pubkey::new_unique();
    let output_program = Pubkey::new_unique();

    create_order(
        &mut order,
        global_config,
        maker,
        1_000,
        2_000,
        Pubkey::default(),
        input_mint,
        output_mint,
        input_program,
        output_program,
        OrderType::Vanilla as u8,
        254,
        100,
        0,
    )
    .unwrap();

    assert_eq!(order.global_config, global_config);
    assert_eq!(order.maker, maker);
    assert_eq!(order.input_mint, input_mint);
    assert_eq!(order.output_mint, output_mint);
    assert_eq!(order.input_mint_program_id, input_program);
    assert_eq!(order.output_mint_program_id, output_program);
    assert_eq!(order.initial_input_amount, 1_000);
    assert_eq!(order.remaining_input_amount, 1_000);
    assert_eq!(order.expected_output_amount, 2_000);
    assert_eq!(order.status, OrderStatus::Active as u8);
    assert_eq!(order.number_of_fills, 0);
    assert_eq!(order.filled_output_amount, 0);
    assert_eq!(order.expiry_timestamp, 0);
}
