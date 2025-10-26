module xrpl_primitives::checks {
    use std::signer;
    use std::error;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_std::table::{Self, Table};

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_CHECK_NOT_FOUND: u64 = 3;
    const E_UNAUTHORIZED: u64 = 4;
    const E_CHECK_EXPIRED: u64 = 5;
    const E_CHECK_CASHED: u64 = 6;
    const E_CHECK_CANCELLED: u64 = 7;
    const E_INVALID_AMOUNT: u64 = 8;
    const E_AMOUNT_EXCEEDS_CHECK: u64 = 9;

    /// Check status
    const STATUS_ACTIVE: u8 = 1;
    const STATUS_CASHED: u8 = 2;
    const STATUS_CANCELLED: u8 = 3;

    /// Check represents a deferred payment (like paper checks)
    struct Check has store {
        sender: address,
        receiver: address,
        amount: u64,
        expiration: u64,
        status: u8,
        cashed_amount: u64,
        created_at: u64,
    }

    /// Global check registry
    struct CheckRegistry has key {
        checks: Table<u64, Check>,
        deposits: Table<u64, Coin<AptosCoin>>,
        next_id: u64,
    }

    /// Events
    #[event]
    struct CheckCreated has drop, store {
        check_id: u64,
        sender: address,
        receiver: address,
        amount: u64,
        expiration: u64,
        timestamp: u64,
    }

    #[event]
    struct CheckCashed has drop, store {
        check_id: u64,
        receiver: address,
        amount: u64,
        timestamp: u64,
    }

    #[event]
    struct CheckCancelled has drop, store {
        check_id: u64,
        sender: address,
        timestamp: u64,
    }

    /// Initialize the check registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<CheckRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, CheckRegistry {
            checks: table::new(),
            deposits: table::new(),
            next_id: 0,
        });
    }

    /// Create a new check
    public entry fun create_check(
        sender: &signer,
        receiver: address,
        amount: u64,
        expiration: u64,
    ) acquires CheckRegistry {
        let sender_addr = signer::address_of(sender);
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<CheckRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let now = timestamp::now_seconds();
        assert!(expiration > now, error::invalid_argument(E_CHECK_EXPIRED));

        let registry = borrow_global_mut<CheckRegistry>(registry_addr);
        let check_id = registry.next_id;
        registry.next_id = check_id + 1;

        // Withdraw coins from sender
        let coins = coin::withdraw<AptosCoin>(sender, amount);

        let check = Check {
            sender: sender_addr,
            receiver,
            amount,
            expiration,
            status: STATUS_ACTIVE,
            cashed_amount: 0,
            created_at: now,
        };

        table::add(&mut registry.checks, check_id, check);
        table::add(&mut registry.deposits, check_id, coins);

        event::emit(CheckCreated {
            check_id,
            sender: sender_addr,
            receiver,
            amount,
            expiration,
            timestamp: now,
        });
    }

    /// Cash a check (receiver only, can cash partial amounts)
    public entry fun cash_check(
        receiver: &signer,
        check_id: u64,
        amount: u64,
    ) acquires CheckRegistry {
        let receiver_addr = signer::address_of(receiver);
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<CheckRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<CheckRegistry>(registry_addr);
        assert!(table::contains(&registry.checks, check_id), error::not_found(E_CHECK_NOT_FOUND));

        let check = table::borrow_mut(&mut registry.checks, check_id);
        assert!(check.receiver == receiver_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(check.status == STATUS_ACTIVE, error::invalid_state(E_CHECK_CASHED));

        let now = timestamp::now_seconds();
        assert!(now < check.expiration, error::invalid_state(E_CHECK_EXPIRED));

        let remaining = check.amount - check.cashed_amount;
        assert!(amount <= remaining, error::invalid_argument(E_AMOUNT_EXCEEDS_CHECK));

        // Extract coins and deposit to receiver
        let deposit = table::borrow_mut(&mut registry.deposits, check_id);
        let payment = coin::extract(deposit, amount);
        coin::deposit(receiver_addr, payment);

        check.cashed_amount = check.cashed_amount + amount;

        // Mark as fully cashed if entire amount claimed
        if (check.cashed_amount >= check.amount) {
            check.status = STATUS_CASHED;
        };

        event::emit(CheckCashed {
            check_id,
            receiver: receiver_addr,
            amount,
            timestamp: now,
        });
    }

    /// Cancel a check (sender only, before it's cashed)
    public entry fun cancel_check(
        sender: &signer,
        check_id: u64,
    ) acquires CheckRegistry {
        let sender_addr = signer::address_of(sender);

        let registry_addr = @xrpl_primitives;
        assert!(exists<CheckRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<CheckRegistry>(registry_addr);
        assert!(table::contains(&registry.checks, check_id), error::not_found(E_CHECK_NOT_FOUND));

        let check = table::borrow_mut(&mut registry.checks, check_id);
        assert!(check.sender == sender_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(check.status == STATUS_ACTIVE, error::invalid_state(E_CHECK_CANCELLED));

        // Return uncashed funds to sender
        let remaining = check.amount - check.cashed_amount;
        if (remaining > 0) {
            let deposit = table::borrow_mut(&mut registry.deposits, check_id);
            let refund = coin::extract(deposit, remaining);
            coin::deposit(sender_addr, refund);
        };

        check.status = STATUS_CANCELLED;

        event::emit(CheckCancelled {
            check_id,
            sender: sender_addr,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Expire a check (anyone can call after expiration to return funds to sender)
    public entry fun expire_check(
        check_id: u64,
    ) acquires CheckRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<CheckRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<CheckRegistry>(registry_addr);
        assert!(table::contains(&registry.checks, check_id), error::not_found(E_CHECK_NOT_FOUND));

        let check = table::borrow_mut(&mut registry.checks, check_id);
        assert!(check.status == STATUS_ACTIVE, error::invalid_state(E_CHECK_CASHED));

        let now = timestamp::now_seconds();
        assert!(now >= check.expiration, error::invalid_state(E_CHECK_EXPIRED));

        // Return uncashed funds to sender
        let remaining = check.amount - check.cashed_amount;
        if (remaining > 0) {
            let deposit = table::borrow_mut(&mut registry.deposits, check_id);
            let refund = coin::extract(deposit, remaining);
            coin::deposit(check.sender, refund);
        };

        check.status = STATUS_CANCELLED;

        event::emit(CheckCancelled {
            check_id,
            sender: check.sender,
            timestamp: now,
        });
    }

    /// View functions
    #[view]
    public fun get_check(check_id: u64): (address, address, u64, u64, u8, u64) acquires CheckRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<CheckRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<CheckRegistry>(registry_addr);
        assert!(table::contains(&registry.checks, check_id), error::not_found(E_CHECK_NOT_FOUND));

        let check = table::borrow(&registry.checks, check_id);
        (check.sender, check.receiver, check.amount, check.expiration, check.status, check.cashed_amount)
    }

    #[view]
    public fun get_remaining_amount(check_id: u64): u64 acquires CheckRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<CheckRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<CheckRegistry>(registry_addr);
        assert!(table::contains(&registry.checks, check_id), error::not_found(E_CHECK_NOT_FOUND));

        let check = table::borrow(&registry.checks, check_id);
        check.amount - check.cashed_amount
    }

    #[view]
    public fun is_expired(check_id: u64): bool acquires CheckRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<CheckRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<CheckRegistry>(registry_addr);
        if (!table::contains(&registry.checks, check_id)) {
            return false
        };

        let check = table::borrow(&registry.checks, check_id);
        timestamp::now_seconds() >= check.expiration
    }
}
