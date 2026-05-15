#[derive(Clone, Copy)]
pub(super) struct SubmitOrderArgs {
    pub(super) input_amount: u64,
    pub(super) output_amount: u64,
    pub(super) order_type: u8,
    pub(super) tp_output_amount: u64,
    pub(super) sl_output_amount: u64,
    pub(super) active_duration_seconds: u64,
}

#[derive(Clone, Copy)]
pub(super) struct SubmitOrderFees {
    pub(super) create_order_fee: u64,
    pub(super) lamports: u64,
}
