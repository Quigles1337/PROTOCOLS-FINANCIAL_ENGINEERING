module xrpl_primitives::checks {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;
    use sui::balance::{Self, Balance};
    use sui::event;

    // Errors
    const ERR_NOT_AUTHORIZED: u64 = 1;
    const ERR_CHECK_EXPIRED: u64 = 2;
    const ERR_INVALID_AMOUNT: u64 = 3;
    const ERR_CHECK_NOT_ACTIVE: u64 = 4;
    const ERR_INSUFFICIENT_REMAINING: u64 = 5;

    // Status
    const STATUS_ACTIVE: u8 = 0;
    const STATUS_CASHED: u8 = 1;
    const STATUS_CANCELLED: u8 = 2;

    // Structs
    public struct Check has key {
        id: UID,
        sender: address,
        receiver: address,
        amount: Balance<SUI>,
        total_amount: u64,
        cashed_amount: u64,
        expiration: u64,
        status: u8,
        created_at: u64,
    }

    // Events
    public struct CheckCreated has copy, drop {
        check_id: address,
        sender: address,
        receiver: address,
        amount: u64,
    }

    public struct CheckCashed has copy, drop {
        check_id: address,
        amount: u64,
    }

    public struct CheckCancelled has copy, drop {
        check_id: address,
    }

    // Create check
    public entry fun create_check(
        deposit: Coin<SUI>,
        receiver: address,
        expiration: u64,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let deposit_value = coin::value(&deposit);
        assert!(deposit_value > 0, ERR_INVALID_AMOUNT);

        let check = Check {
            id: object::new(ctx),
            sender,
            receiver,
            amount: coin::into_balance(deposit),
            total_amount: deposit_value,
            cashed_amount: 0,
            expiration,
            status: STATUS_ACTIVE,
            created_at: tx_context::epoch(ctx),
        };

        let check_id = object::uid_to_address(&check.id);

        event::emit(CheckCreated {
            check_id,
            sender,
            receiver,
            amount: deposit_value,
        });

        transfer::share_object(check);
    }

    // Cash check (partial cashing supported)
    public entry fun cash_check(
        check: &mut Check,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let receiver = tx_context::sender(ctx);
        assert!(receiver == check.receiver, ERR_NOT_AUTHORIZED);
        assert!(check.status == STATUS_ACTIVE, ERR_CHECK_NOT_ACTIVE);
        assert!(tx_context::epoch(ctx) < check.expiration, ERR_CHECK_EXPIRED);

        let remaining = check.total_amount - check.cashed_amount;
        assert!(amount > 0 && amount <= remaining, ERR_INSUFFICIENT_REMAINING);

        check.cashed_amount = check.cashed_amount + amount;

        if (check.cashed_amount == check.total_amount) {
            check.status = STATUS_CASHED;
        };

        let cash_balance = balance::split(&mut check.amount, amount);
        let cash_coin = coin::from_balance(cash_balance, ctx);

        event::emit(CheckCashed {
            check_id: object::uid_to_address(&check.id),
            amount,
        });

        transfer::public_transfer(cash_coin, receiver);
    }

    // Cancel check
    public entry fun cancel_check(
        check: &mut Check,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(sender == check.sender, ERR_NOT_AUTHORIZED);
        assert!(check.status == STATUS_ACTIVE, ERR_CHECK_NOT_ACTIVE);

        check.status = STATUS_CANCELLED;

        let remaining = balance::value(&check.amount);
        if (remaining > 0) {
            let refund_balance = balance::withdraw_all(&mut check.amount);
            let refund_coin = coin::from_balance(refund_balance, ctx);
            transfer::public_transfer(refund_coin, sender);
        };

        event::emit(CheckCancelled {
            check_id: object::uid_to_address(&check.id),
        });
    }

    // Expire check
    public entry fun expire_check(
        check: &mut Check,
        ctx: &mut TxContext
    ) {
        assert!(check.status == STATUS_ACTIVE, ERR_CHECK_NOT_ACTIVE);
        assert!(tx_context::epoch(ctx) >= check.expiration, ERR_CHECK_EXPIRED);

        check.status = STATUS_CANCELLED;

        let remaining = balance::value(&check.amount);
        if (remaining > 0) {
            let refund_balance = balance::withdraw_all(&mut check.amount);
            let refund_coin = coin::from_balance(refund_balance, ctx);
            transfer::public_transfer(refund_coin, check.sender);
        };

        event::emit(CheckCancelled {
            check_id: object::uid_to_address(&check.id),
        });
    }

    // View functions
    public fun get_remaining_amount(check: &Check): u64 {
        check.total_amount - check.cashed_amount
    }

    public fun get_status(check: &Check): u8 {
        check.status
    }
}
