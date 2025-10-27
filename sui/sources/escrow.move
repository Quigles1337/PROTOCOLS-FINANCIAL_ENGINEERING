module xrpl_primitives::escrow {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;
    use sui::balance::{Self, Balance};
    use sui::event;
    use sui::hash;
    use std::option::{Self, Option};

    // Errors
    const ERR_NOT_AUTHORIZED: u64 = 1;
    const ERR_NOT_RELEASED: u64 = 2;
    const ERR_WRONG_PREIMAGE: u64 = 3;
    const ERR_INVALID_AMOUNT: u64 = 4;
    const ERR_TOO_EARLY: u64 = 5;
    const ERR_ALREADY_EXECUTED: u64 = 6;

    // Status
    const STATUS_ACTIVE: u8 = 0;
    const STATUS_EXECUTED: u8 = 1;
    const STATUS_CANCELLED: u8 = 2;

    // Structs
    public struct Escrow has key {
        id: UID,
        sender: address,
        receiver: address,
        amount: Balance<SUI>,
        release_time: u64,
        cancel_time: u64,
        condition_hash: Option<vector<u8>>,
        status: u8,
    }

    // Events
    public struct EscrowCreated has copy, drop {
        escrow_id: address,
        sender: address,
        receiver: address,
        amount: u64,
    }

    public struct EscrowExecuted has copy, drop {
        escrow_id: address,
    }

    public struct EscrowCancelled has copy, drop {
        escrow_id: address,
    }

    // Create time-locked escrow
    public entry fun create_time_locked(
        deposit: Coin<SUI>,
        receiver: address,
        release_time: u64,
        cancel_time: u64,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let deposit_value = coin::value(&deposit);
        assert!(deposit_value > 0, ERR_INVALID_AMOUNT);

        let escrow = Escrow {
            id: object::new(ctx),
            sender,
            receiver,
            amount: coin::into_balance(deposit),
            release_time,
            cancel_time,
            condition_hash: option::none(),
            status: STATUS_ACTIVE,
        };

        let escrow_id = object::uid_to_address(&escrow.id);

        event::emit(EscrowCreated {
            escrow_id,
            sender,
            receiver,
            amount: deposit_value,
        });

        transfer::share_object(escrow);
    }

    // Create hash-locked escrow (HTLC)
    public entry fun create_hash_locked(
        deposit: Coin<SUI>,
        receiver: address,
        release_time: u64,
        cancel_time: u64,
        condition_hash: vector<u8>,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let deposit_value = coin::value(&deposit);
        assert!(deposit_value > 0, ERR_INVALID_AMOUNT);

        let escrow = Escrow {
            id: object::new(ctx),
            sender,
            receiver,
            amount: coin::into_balance(deposit),
            release_time,
            cancel_time,
            condition_hash: option::some(condition_hash),
            status: STATUS_ACTIVE,
        };

        let escrow_id = object::uid_to_address(&escrow.id);

        event::emit(EscrowCreated {
            escrow_id,
            sender,
            receiver,
            amount: deposit_value,
        });

        transfer::share_object(escrow);
    }

    // Execute escrow
    public entry fun execute_escrow(
        escrow: &mut Escrow,
        preimage: Option<vector<u8>>,
        ctx: &mut TxContext
    ) {
        let receiver = tx_context::sender(ctx);
        assert!(receiver == escrow.receiver, ERR_NOT_AUTHORIZED);
        assert!(escrow.status == STATUS_ACTIVE, ERR_ALREADY_EXECUTED);
        assert!(tx_context::epoch(ctx) >= escrow.release_time, ERR_TOO_EARLY);

        // Verify hash condition if present
        if (option::is_some(&escrow.condition_hash)) {
            assert!(option::is_some(&preimage), ERR_WRONG_PREIMAGE);
            let hash = option::borrow(&escrow.condition_hash);
            let pre = option::borrow(&preimage);
            let computed_hash = hash::keccak256(pre);
            assert!(&computed_hash == hash, ERR_WRONG_PREIMAGE);
        };

        escrow.status = STATUS_EXECUTED;

        let amount_value = balance::value(&escrow.amount);
        let release_balance = balance::withdraw_all(&mut escrow.amount);
        let release_coin = coin::from_balance(release_balance, ctx);

        event::emit(EscrowExecuted {
            escrow_id: object::uid_to_address(&escrow.id),
        });

        transfer::public_transfer(release_coin, receiver);
    }

    // Cancel escrow
    public entry fun cancel_escrow(
        escrow: &mut Escrow,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(sender == escrow.sender, ERR_NOT_AUTHORIZED);
        assert!(escrow.status == STATUS_ACTIVE, ERR_ALREADY_EXECUTED);
        assert!(tx_context::epoch(ctx) >= escrow.cancel_time, ERR_TOO_EARLY);

        escrow.status = STATUS_CANCELLED;

        let refund_balance = balance::withdraw_all(&mut escrow.amount);
        let refund_coin = coin::from_balance(refund_balance, ctx);

        event::emit(EscrowCancelled {
            escrow_id: object::uid_to_address(&escrow.id),
        });

        transfer::public_transfer(refund_coin, sender);
    }

    // View functions
    public fun is_executable(escrow: &Escrow, current_epoch: u64): bool {
        escrow.status == STATUS_ACTIVE && current_epoch >= escrow.release_time
    }

    public fun get_amount(escrow: &Escrow): u64 {
        balance::value(&escrow.amount)
    }
}
