use crate::state::{Order, OrderDisplay};

pub(super) fn order_display(
    order: &Order,
    on_event_output_amount_filled: u64,
    on_event_tip_amount: u64,
) -> OrderDisplay {
    OrderDisplay {
        initial_input_amount: order.initial_input_amount,
        expected_output_amount: order.expected_output_amount,
        remaining_input_amount: order.remaining_input_amount,
        filled_output_amount: order.filled_output_amount,
        tip_amount: order.tip_amount,
        number_of_fills: order.number_of_fills,
        on_event_output_amount_filled,
        on_event_tip_amount,
        order_type: order.order_type,
        status: order.status,
        last_updated_timestamp: order.last_updated_timestamp,
    }
}
