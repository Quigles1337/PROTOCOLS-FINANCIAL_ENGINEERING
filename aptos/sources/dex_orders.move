module xrpl_primitives::dex_orders {
    use std::signer;
    use std::error;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info::{Self, TypeInfo};

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_ORDER_NOT_FOUND: u64 = 3;
    const E_UNAUTHORIZED: u64 = 4;
    const E_ORDER_NOT_OPEN: u64 = 5;
    const E_INVALID_AMOUNT: u64 = 6;
    const E_SAME_ASSET: u64 = 7;
    const E_INSUFFICIENT_FILL: u64 = 8;

    /// Order status
    const STATUS_OPEN: u8 = 1;
    const STATUS_PARTIALLY_FILLED: u8 = 2;
    const STATUS_FILLED: u8 = 3;
    const STATUS_CANCELLED: u8 = 4;

    /// Order in the DEX orderbook
    struct Order has store, copy, drop {
        maker: address,
        sell_asset: TypeInfo,
        buy_asset: TypeInfo,
        sell_amount: u64,
        buy_amount: u64,
        filled: u64,
        status: u8,
        created_at: u64,
    }

    /// Global orderbook
    struct Orderbook has key {
        orders: Table<u64, Order>,
        next_id: u64,
    }

    /// Events
    #[event]
    struct OrderPlaced has drop, store {
        order_id: u64,
        maker: address,
        sell_amount: u64,
        buy_amount: u64,
        timestamp: u64,
    }

    #[event]
    struct OrderFilled has drop, store {
        order_id: u64,
        taker: address,
        amount: u64,
        payment: u64,
        timestamp: u64,
    }

    #[event]
    struct OrderCancelled has drop, store {
        order_id: u64,
        maker: address,
        timestamp: u64,
    }

    /// Initialize the orderbook
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<Orderbook>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, Orderbook {
            orders: table::new(),
            next_id: 0,
        });
    }

    /// Place a limit order
    public fun place_order<SellAsset, BuyAsset>(
        maker: &signer,
        sell_amount: u64,
        buy_amount: u64,
    ): u64 acquires Orderbook {
        let maker_addr = signer::address_of(maker);
        assert!(sell_amount > 0, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(buy_amount > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let sell_asset = type_info::type_of<SellAsset>();
        let buy_asset = type_info::type_of<BuyAsset>();
        assert!(sell_asset != buy_asset, error::invalid_argument(E_SAME_ASSET));

        let registry_addr = @xrpl_primitives;
        assert!(exists<Orderbook>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let orderbook = borrow_global_mut<Orderbook>(registry_addr);
        let order_id = orderbook.next_id;
        orderbook.next_id = order_id + 1;

        let order = Order {
            maker: maker_addr,
            sell_asset,
            buy_asset,
            sell_amount,
            buy_amount,
            filled: 0,
            status: STATUS_OPEN,
            created_at: timestamp::now_seconds(),
        };

        table::add(&mut orderbook.orders, order_id, order);

        event::emit(OrderPlaced {
            order_id,
            maker: maker_addr,
            sell_amount,
            buy_amount,
            timestamp: timestamp::now_seconds(),
        });

        order_id
    }

    /// Fill an order (partial fills supported)
    public fun fill_order<SellAsset, BuyAsset>(
        taker: &signer,
        order_id: u64,
        fill_amount: u64,
    ): u64 acquires Orderbook {
        let taker_addr = signer::address_of(taker);
        assert!(fill_amount > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<Orderbook>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let orderbook = borrow_global_mut<Orderbook>(registry_addr);
        assert!(table::contains(&orderbook.orders, order_id), error::not_found(E_ORDER_NOT_FOUND));

        let order = table::borrow_mut(&mut orderbook.orders, order_id);
        assert!(
            order.status == STATUS_OPEN || order.status == STATUS_PARTIALLY_FILLED,
            error::invalid_state(E_ORDER_NOT_OPEN)
        );

        let remaining = order.sell_amount - order.filled;
        let actual_fill = if (fill_amount > remaining) { remaining } else { fill_amount };

        // Calculate proportional payment
        let required_payment = ((actual_fill as u128) * (order.buy_amount as u128) / (order.sell_amount as u128) as u64);

        order.filled = order.filled + actual_fill;

        // Update status
        if (order.filled >= order.sell_amount) {
            order.status = STATUS_FILLED;
        } else {
            order.status = STATUS_PARTIALLY_FILLED;
        };

        event::emit(OrderFilled {
            order_id,
            taker: taker_addr,
            amount: actual_fill,
            payment: required_payment,
            timestamp: timestamp::now_seconds(),
        });

        required_payment
    }

    /// Cancel an order
    public entry fun cancel_order(
        maker: &signer,
        order_id: u64,
    ) acquires Orderbook {
        let maker_addr = signer::address_of(maker);

        let registry_addr = @xrpl_primitives;
        assert!(exists<Orderbook>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let orderbook = borrow_global_mut<Orderbook>(registry_addr);
        assert!(table::contains(&orderbook.orders, order_id), error::not_found(E_ORDER_NOT_FOUND));

        let order = table::borrow_mut(&mut orderbook.orders, order_id);
        assert!(order.maker == maker_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(
            order.status == STATUS_OPEN || order.status == STATUS_PARTIALLY_FILLED,
            error::invalid_state(E_ORDER_NOT_OPEN)
        );

        order.status = STATUS_CANCELLED;

        event::emit(OrderCancelled {
            order_id,
            maker: maker_addr,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// View functions
    #[view]
    public fun get_order(order_id: u64): (address, u64, u64, u64, u8) acquires Orderbook {
        let registry_addr = @xrpl_primitives;
        assert!(exists<Orderbook>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let orderbook = borrow_global<Orderbook>(registry_addr);
        assert!(table::contains(&orderbook.orders, order_id), error::not_found(E_ORDER_NOT_FOUND));

        let order = table::borrow(&orderbook.orders, order_id);
        (order.maker, order.sell_amount, order.buy_amount, order.filled, order.status)
    }

    #[view]
    public fun get_price(order_id: u64): (u64, u64) acquires Orderbook {
        let registry_addr = @xrpl_primitives;
        assert!(exists<Orderbook>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let orderbook = borrow_global<Orderbook>(registry_addr);
        assert!(table::contains(&orderbook.orders, order_id), error::not_found(E_ORDER_NOT_FOUND));

        let order = table::borrow(&orderbook.orders, order_id);
        (order.buy_amount, order.sell_amount)
    }

    #[view]
    public fun get_remaining(order_id: u64): u64 acquires Orderbook {
        let registry_addr = @xrpl_primitives;
        assert!(exists<Orderbook>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let orderbook = borrow_global<Orderbook>(registry_addr);
        assert!(table::contains(&orderbook.orders, order_id), error::not_found(E_ORDER_NOT_FOUND));

        let order = table::borrow(&orderbook.orders, order_id);
        order.sell_amount - order.filled
    }

    #[view]
    public fun is_fillable(order_id: u64): bool acquires Orderbook {
        let registry_addr = @xrpl_primitives;
        if (!exists<Orderbook>(registry_addr)) {
            return false
        };

        let orderbook = borrow_global<Orderbook>(registry_addr);
        if (!table::contains(&orderbook.orders, order_id)) {
            return false
        };

        let order = table::borrow(&orderbook.orders, order_id);
        order.status == STATUS_OPEN || order.status == STATUS_PARTIALLY_FILLED
    }
}
