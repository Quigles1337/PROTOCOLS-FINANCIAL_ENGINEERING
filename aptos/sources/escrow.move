module xrpl_primitives::escrow {
    use std::signer;
    use std::error;
    use std::vector;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_std::table::{Self, Table};
    use aptos_std::aptos_hash;

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_ESCROW_NOT_FOUND: u64 = 3;
    const E_UNAUTHORIZED: u64 = 4;
    const E_NOT_RELEASED: u64 = 5;
    const E_ALREADY_EXECUTED: u64 = 6;
    const E_INVALID_CONDITION: u64 = 7;
    const E_WRONG_PREIMAGE: u64 = 8;
    const E_CANCEL_TOO_EARLY: u64 = 9;
    const E_INVALID_AMOUNT: u64 = 10;

    /// Escrow status
    const STATUS_ACTIVE: u8 = 1;
    const STATUS_EXECUTED: u8 = 2;
    const STATUS_CANCELLED: u8 = 3;

    /// Escrow for time-locked and hash-locked conditional payments (HTLC)
    struct Escrow has store {
        sender: address,
        receiver: address,
        amount: u64,
        release_time: u64,
        cancel_time: u64,
        condition_hash: vector<u8>,  // Hash lock (empty if no condition)
        status: u8,
        created_at: u64,
    }

    /// Global escrow registry
    struct EscrowRegistry has key {
        escrows: Table<u64, Escrow>,
        deposits: Table<u64, Coin<AptosCoin>>,
        next_id: u64,
    }

    /// Events
    #[event]
    struct EscrowCreated has drop, store {
        escrow_id: u64,
        sender: address,
        receiver: address,
        amount: u64,
        release_time: u64,
        cancel_time: u64,
        has_condition: bool,
        timestamp: u64,
    }

    #[event]
    struct EscrowExecuted has drop, store {
        escrow_id: u64,
        receiver: address,
        amount: u64,
        timestamp: u64,
    }

    #[event]
    struct EscrowCancelled has drop, store {
        escrow_id: u64,
        sender: address,
        amount: u64,
        timestamp: u64,
    }

    /// Initialize the escrow registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<EscrowRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, EscrowRegistry {
            escrows: table::new(),
            deposits: table::new(),
            next_id: 0,
        });
    }

    /// Create time-locked escrow
    public entry fun create_time_locked(
        sender: &signer,
        receiver: address,
        amount: u64,
        release_time: u64,
        cancel_time: u64,
    ) acquires EscrowRegistry {
        let empty_hash = vector::empty<u8>();
        create_escrow_internal(sender, receiver, amount, release_time, cancel_time, empty_hash);
    }

    /// Create hash-locked escrow (HTLC)
    public entry fun create_hash_locked(
        sender: &signer,
        receiver: address,
        amount: u64,
        release_time: u64,
        cancel_time: u64,
        condition_hash: vector<u8>,
    ) acquires EscrowRegistry {
        assert!(vector::length(&condition_hash) == 32, error::invalid_argument(E_INVALID_CONDITION));
        create_escrow_internal(sender, receiver, amount, release_time, cancel_time, condition_hash);
    }

    /// Internal escrow creation
    fun create_escrow_internal(
        sender: &signer,
        receiver: address,
        amount: u64,
        release_time: u64,
        cancel_time: u64,
        condition_hash: vector<u8>,
    ) acquires EscrowRegistry {
        let sender_addr = signer::address_of(sender);
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<EscrowRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let now = timestamp::now_seconds();
        assert!(release_time >= now, error::invalid_argument(E_NOT_RELEASED));
        assert!(cancel_time > release_time, error::invalid_argument(E_CANCEL_TOO_EARLY));

        let registry = borrow_global_mut<EscrowRegistry>(registry_addr);
        let escrow_id = registry.next_id;
        registry.next_id = escrow_id + 1;

        // Withdraw coins from sender
        let coins = coin::withdraw<AptosCoin>(sender, amount);

        let has_condition = !vector::is_empty(&condition_hash);

        let escrow = Escrow {
            sender: sender_addr,
            receiver,
            amount,
            release_time,
            cancel_time,
            condition_hash,
            status: STATUS_ACTIVE,
            created_at: now,
        };

        table::add(&mut registry.escrows, escrow_id, escrow);
        table::add(&mut registry.deposits, escrow_id, coins);

        event::emit(EscrowCreated {
            escrow_id,
            sender: sender_addr,
            receiver,
            amount,
            release_time,
            cancel_time,
            has_condition,
            timestamp: now,
        });
    }

    /// Execute escrow (with optional preimage for hash-locked)
    public entry fun execute_escrow(
        receiver: &signer,
        escrow_id: u64,
        preimage: vector<u8>,
    ) acquires EscrowRegistry {
        let receiver_addr = signer::address_of(receiver);

        let registry_addr = @xrpl_primitives;
        assert!(exists<EscrowRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<EscrowRegistry>(registry_addr);
        assert!(table::contains(&registry.escrows, escrow_id), error::not_found(E_ESCROW_NOT_FOUND));

        let escrow = table::borrow_mut(&mut registry.escrows, escrow_id);
        assert!(escrow.receiver == receiver_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(escrow.status == STATUS_ACTIVE, error::invalid_state(E_ALREADY_EXECUTED));

        let now = timestamp::now_seconds();
        assert!(now >= escrow.release_time, error::invalid_state(E_NOT_RELEASED));
        assert!(now < escrow.cancel_time, error::invalid_state(E_CANCEL_TOO_EARLY));

        // Verify hash condition if present
        if (!vector::is_empty(&escrow.condition_hash)) {
            let hash = aptos_hash::sha3_256(preimage);
            assert!(hash == escrow.condition_hash, error::invalid_argument(E_WRONG_PREIMAGE));
        };

        // Transfer funds to receiver
        let deposit = table::borrow_mut(&mut registry.deposits, escrow_id);
        let payment = coin::extract_all(deposit);
        coin::deposit(receiver_addr, payment);

        escrow.status = STATUS_EXECUTED;

        event::emit(EscrowExecuted {
            escrow_id,
            receiver: receiver_addr,
            amount: escrow.amount,
            timestamp: now,
        });
    }

    /// Cancel escrow (sender only, after cancel time)
    public entry fun cancel_escrow(
        sender: &signer,
        escrow_id: u64,
    ) acquires EscrowRegistry {
        let sender_addr = signer::address_of(sender);

        let registry_addr = @xrpl_primitives;
        assert!(exists<EscrowRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<EscrowRegistry>(registry_addr);
        assert!(table::contains(&registry.escrows, escrow_id), error::not_found(E_ESCROW_NOT_FOUND));

        let escrow = table::borrow_mut(&mut registry.escrows, escrow_id);
        assert!(escrow.sender == sender_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(escrow.status == STATUS_ACTIVE, error::invalid_state(E_ALREADY_EXECUTED));

        let now = timestamp::now_seconds();
        assert!(now >= escrow.cancel_time, error::invalid_state(E_CANCEL_TOO_EARLY));

        // Return funds to sender
        let deposit = table::borrow_mut(&mut registry.deposits, escrow_id);
        let refund = coin::extract_all(deposit);
        coin::deposit(sender_addr, refund);

        escrow.status = STATUS_CANCELLED;

        event::emit(EscrowCancelled {
            escrow_id,
            sender: sender_addr,
            amount: escrow.amount,
            timestamp: now,
        });
    }

    /// View functions
    #[view]
    public fun get_escrow(escrow_id: u64): (address, address, u64, u64, u64, bool, u8) acquires EscrowRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<EscrowRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<EscrowRegistry>(registry_addr);
        assert!(table::contains(&registry.escrows, escrow_id), error::not_found(E_ESCROW_NOT_FOUND));

        let escrow = table::borrow(&registry.escrows, escrow_id);
        let has_condition = !vector::is_empty(&escrow.condition_hash);
        (escrow.sender, escrow.receiver, escrow.amount, escrow.release_time, escrow.cancel_time, has_condition, escrow.status)
    }

    #[view]
    public fun is_executable(escrow_id: u64): bool acquires EscrowRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<EscrowRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<EscrowRegistry>(registry_addr);
        if (!table::contains(&registry.escrows, escrow_id)) {
            return false
        };

        let escrow = table::borrow(&registry.escrows, escrow_id);
        let now = timestamp::now_seconds();
        escrow.status == STATUS_ACTIVE && now >= escrow.release_time && now < escrow.cancel_time
    }

    #[view]
    public fun is_cancellable(escrow_id: u64): bool acquires EscrowRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<EscrowRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<EscrowRegistry>(registry_addr);
        if (!table::contains(&registry.escrows, escrow_id)) {
            return false
        };

        let escrow = table::borrow(&registry.escrows, escrow_id);
        let now = timestamp::now_seconds();
        escrow.status == STATUS_ACTIVE && now >= escrow.cancel_time
    }
}
