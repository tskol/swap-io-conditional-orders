use anchor_lang::prelude::Pubkey;
use limo::{
    operations::create_order,
    state::{Order, OrderStatus, OrderType},
};

struct VanillaOrderFixture {
    global_config: Pubkey,
    maker: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
    input_program: Pubkey,
    output_program: Pubkey,
}

impl VanillaOrderFixture {
    fn new() -> Self {
        Self {
            global_config: Pubkey::new_unique(),
            maker: Pubkey::new_unique(),
            input_mint: Pubkey::new_unique(),
            output_mint: Pubkey::new_unique(),
            input_program: Pubkey::new_unique(),
            output_program: Pubkey::new_unique(),
        }
    }

    fn create_order(&self, order: &mut Order, active_duration_seconds: u64) {
        create_order(
            order,
            self.global_config,
            self.maker,
            1_000,
            2_000,
            Pubkey::default(),
            self.input_mint,
            self.output_mint,
            self.input_program,
            self.output_program,
            OrderType::Vanilla as u8,
            254,
            100,
            active_duration_seconds,
        )
        .unwrap();
    }
}

#[test]
fn create_order_initializes_vanilla_order_fields() {
    let mut order = Order::default();
    let fixture = VanillaOrderFixture::new();

    fixture.create_order(&mut order, 0);

    assert_eq!(order.global_config, fixture.global_config);
    assert_eq!(order.maker, fixture.maker);
    assert_eq!(order.input_mint, fixture.input_mint);
    assert_eq!(order.output_mint, fixture.output_mint);
    assert_eq!(order.input_mint_program_id, fixture.input_program);
    assert_eq!(order.output_mint_program_id, fixture.output_program);
    assert_eq!(order.initial_input_amount, 1_000);
    assert_eq!(order.remaining_input_amount, 1_000);
    assert_eq!(order.expected_output_amount, 2_000);
    assert_eq!(order.status, OrderStatus::Active as u8);
    assert_eq!(order.number_of_fills, 0);
    assert_eq!(order.filled_output_amount, 0);
    assert_eq!(order.expiry_timestamp, 0);
}

#[test]
fn create_order_sets_expiry_when_active_duration_is_nonzero() {
    let mut order = Order::default();
    let fixture = VanillaOrderFixture::new();

    fixture.create_order(&mut order, 50);

    assert_eq!(order.last_updated_timestamp, 100);
    assert_eq!(order.expiry_timestamp, 150);
}
