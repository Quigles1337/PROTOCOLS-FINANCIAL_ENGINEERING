module xrpl_primitives::dex_orders {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;
    use sui::balance::{Self, Balance};
    use sui::event;
    use std::string::String;

    // Errors
    const ERR_NOT_AUTHORIZED: u64 = 1;
    const ERR_ORDER_NOT_AVAILABLE: u64 = 2;
    const ERR_INVALID_AMOUNT: u64 = 3;
    const ERR_INVALID_FILL: u64 = 4;

    // Status
    const STATUS_OPEN: u8 = 0;
    const STATUS_PARTIALLY_FILLED: u8 = 1;
    const STATUS_FILLED: u8 = 2;
    const STATUS_CANCELLED: u8 = 3;

    // Structs
    public struct Order has key {
        id: UID,
        creator: address,
        sell_asset: String,
        buy_asset: String,
        sell_amount: Balance<SUI>,
        total_sell_amount: u64,
        buy_amount: u64,
        filled_amount: u64,
        status: u8,
        created_at: u64,
    }

    // Events
    public struct OrderPlaced has copy, drop {
        order_id: address,
        creator: address,
        sell_amount: u64,
        buy_amount: u64,
    }

    public struct OrderFilled has copy, drop {
        order_id: address,
        fill_amount: u64,
        payment: u64,
    }

    public struct OrderCancelled has copy, drop {
        order_id: address,
    }

    // Place order
    public entry fun place_order(
        sell_deposit: Coin<SUI>,
        sell_asset: String,
        buy_asset: String,
        buy_amount: u64,
        ctx: &mut TxContext
    ) {
        let creator = tx_context::sender(ctx);
        let sell_amount = coin::value(&sell_deposit);
        assert!(sell_amount > 0 && buy_amount > 0, ERR_INVALID_AMOUNT);

        let order = Order {
            id: object::new(ctx),
            creator,
            sell_asset,
            buy_asset,
            sell_amount: coin::into_balance(sell_deposit),
            total_sell_amount: sell_amount,
            buy_amount,
            filled_amount: 0,
            status: STATUS_OPEN,
            created_at: tx_context::epoch(ctx),
        };

        let order_id = object::uid_to_address(&order.id);

        event::emit(OrderPlaced {
            order_id,
            creator,
            sell_amount,
            buy_amount,
        });

        transfer::share_object(order);
    }

    // Fill order
    public entry fun fill_order(
        order: &mut Order,
        fill_amount: u64,
        payment: Coin<SUI>,
        ctx: &mut TxContext
    ) {
        assert!(
            order.status == STATUS_OPEN || order.status == STATUS_PARTIALLY_FILLED,
            ERR_ORDER_NOT_AVAILABLE
        );

        let remaining = order.total_sell_amount - order.filled_amount;
        assert!(fill_amount > 0 && fill_amount <= remaining, ERR_INVALID_FILL);

        // Calculate proportional payment
        let required_payment = (fill_amount * order.buy_amount) / order.total_sell_amount;
        let payment_value = coin::value(&payment);
        assert!(payment_value == required_payment, ERR_INVALID_AMOUNT);

        order.filled_amount = order.filled_amount + fill_amount;

        if (order.filled_amount == order.total_sell_amount) {
            order.status = STATUS_FILLED;
        } else {
            order.status = STATUS_PARTIALLY_FILLED;
        };

        // Transfer filled amount to filler
        let fill_balance = balance::split(&mut order.sell_amount, fill_amount);
        let fill_coin = coin::from_balance(fill_balance, ctx);
        let filler = tx_context::sender(ctx);
        transfer::public_transfer(fill_coin, filler);

        // Transfer payment to order creator
        transfer::public_transfer(payment, order.creator);

        event::emit(OrderFilled {
            order_id: object::uid_to_address(&order.id),
            fill_amount,
            payment: payment_value,
        });
    }

    // Cancel order
    public entry fun cancel_order(
        order: &mut Order,
        ctx: &mut TxContext
    ) {
        let creator = tx_context::sender(ctx);
        assert!(creator == order.creator, ERR_NOT_AUTHORIZED);
        assert!(
            order.status == STATUS_OPEN || order.status == STATUS_PARTIALLY_FILLED,
            ERR_ORDER_NOT_AVAILABLE
        );

        order.status = STATUS_CANCELLED;

        let remaining = balance::value(&order.sell_amount);
        if (remaining > 0) {
            let refund_balance = balance::withdraw_all(&mut order.sell_amount);
            let refund_coin = coin::from_balance(refund_balance, ctx);
            transfer::public_transfer(refund_coin, creator);
        };

        event::emit(OrderCancelled {
            order_id: object::uid_to_address(&order.id),
        });
    }

    // View functions
    public fun get_remaining_amount(order: &Order): u64 {
        order.total_sell_amount - order.filled_amount
    }

    public fun get_status(order: &Order): u8 {
        order.status
    }
}
