module xrpl_primitives::trust_lines {
    use std::signer;
    use std::error;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info;

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_TRUST_LINE_EXISTS: u64 = 3;
    const E_TRUST_LINE_NOT_FOUND: u64 = 4;
    const E_INSUFFICIENT_LIMIT: u64 = 5;
    const E_UNAUTHORIZED: u64 = 6;
    const E_SAME_ACCOUNT: u64 = 7;
    const E_BALANCE_EXCEEDED: u64 = 8;

    /// TrustLine represents a bilateral credit line between two accounts
    struct TrustLine has store, copy, drop {
        account1: address,
        account2: address,
        limit1: u64,  // Credit limit account1 extends to account2
        limit2: u64,  // Credit limit account2 extends to account1
        balance: u64, // Current balance (account1 owes account2)
        is_negative: bool, // True if balance is negative (account2 owes account1)
        active: bool,
        created_at: u64,
    }

    /// Global storage for trust lines
    struct TrustLineRegistry has key {
        lines: Table<u128, TrustLine>, // key = hash(account1, account2)
        next_id: u64,
    }

    /// Events
    #[event]
    struct TrustLineCreated has drop, store {
        id: u128,
        account1: address,
        account2: address,
        limit1: u64,
        limit2: u64,
        timestamp: u64,
    }

    #[event]
    struct TrustLineUpdated has drop, store {
        id: u128,
        new_limit1: u64,
        new_limit2: u64,
        timestamp: u64,
    }

    #[event]
    struct PaymentRippled has drop, store {
        id: u128,
        from: address,
        to: address,
        amount: u64,
        new_balance: u64,
        timestamp: u64,
    }

    #[event]
    struct TrustLineClosed has drop, store {
        id: u128,
        account1: address,
        account2: address,
        timestamp: u64,
    }

    /// Initialize the trust line registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<TrustLineRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, TrustLineRegistry {
            lines: table::new(),
            next_id: 0,
        });
    }

    /// Create a new trust line between two accounts
    public entry fun create_trust_line(
        creator: &signer,
        counterparty: address,
        limit1: u64,
        limit2: u64,
    ) acquires TrustLineRegistry {
        let creator_addr = signer::address_of(creator);
        assert!(creator_addr != counterparty, error::invalid_argument(E_SAME_ACCOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<TrustLineRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<TrustLineRegistry>(registry_addr);
        let id = generate_id(creator_addr, counterparty);

        assert!(!table::contains(&registry.lines, id), error::already_exists(E_TRUST_LINE_EXISTS));

        let trust_line = TrustLine {
            account1: creator_addr,
            account2: counterparty,
            limit1,
            limit2,
            balance: 0,
            is_negative: false,
            active: true,
            created_at: timestamp::now_seconds(),
        };

        table::add(&mut registry.lines, id, trust_line);

        event::emit(TrustLineCreated {
            id,
            account1: creator_addr,
            account2: counterparty,
            limit1,
            limit2,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Update trust line limits
    public entry fun update_limits(
        account: &signer,
        counterparty: address,
        new_limit: u64,
    ) acquires TrustLineRegistry {
        let addr = signer::address_of(account);
        let registry_addr = @xrpl_primitives;
        assert!(exists<TrustLineRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<TrustLineRegistry>(registry_addr);
        let id = generate_id(addr, counterparty);

        assert!(table::contains(&registry.lines, id), error::not_found(E_TRUST_LINE_NOT_FOUND));

        let trust_line = table::borrow_mut(&mut registry.lines, id);

        // Update the appropriate limit based on which account is calling
        if (trust_line.account1 == addr) {
            trust_line.limit1 = new_limit;
        } else {
            trust_line.limit2 = new_limit;
        };

        event::emit(TrustLineUpdated {
            id,
            new_limit1: trust_line.limit1,
            new_limit2: trust_line.limit2,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Ripple payment through trust line
    public entry fun ripple_payment(
        sender: &signer,
        receiver: address,
        amount: u64,
    ) acquires TrustLineRegistry {
        let sender_addr = signer::address_of(sender);
        let registry_addr = @xrpl_primitives;
        assert!(exists<TrustLineRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<TrustLineRegistry>(registry_addr);
        let id = generate_id(sender_addr, receiver);

        assert!(table::contains(&registry.lines, id), error::not_found(E_TRUST_LINE_NOT_FOUND));

        let trust_line = table::borrow_mut(&mut registry.lines, id);
        assert!(trust_line.active, error::invalid_state(E_TRUST_LINE_NOT_FOUND));

        // Determine direction and update balance
        if (trust_line.account1 == sender_addr) {
            // account1 -> account2: increase balance
            if (trust_line.is_negative) {
                // Balance was negative, reduce it
                if (amount >= trust_line.balance) {
                    trust_line.balance = amount - trust_line.balance;
                    trust_line.is_negative = false;
                } else {
                    trust_line.balance = trust_line.balance - amount;
                }
            } else {
                trust_line.balance = trust_line.balance + amount;
                assert!(trust_line.balance <= trust_line.limit1, error::invalid_argument(E_BALANCE_EXCEEDED));
            }
        } else {
            // account2 -> account1: decrease balance (or make negative)
            if (!trust_line.is_negative) {
                if (amount >= trust_line.balance) {
                    trust_line.balance = amount - trust_line.balance;
                    trust_line.is_negative = true;
                    assert!(trust_line.balance <= trust_line.limit2, error::invalid_argument(E_BALANCE_EXCEEDED));
                } else {
                    trust_line.balance = trust_line.balance - amount;
                }
            } else {
                trust_line.balance = trust_line.balance + amount;
                assert!(trust_line.balance <= trust_line.limit2, error::invalid_argument(E_BALANCE_EXCEEDED));
            }
        };

        event::emit(PaymentRippled {
            id,
            from: sender_addr,
            to: receiver,
            amount,
            new_balance: trust_line.balance,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Close a trust line (only if balance is zero)
    public entry fun close_trust_line(
        account: &signer,
        counterparty: address,
    ) acquires TrustLineRegistry {
        let addr = signer::address_of(account);
        let registry_addr = @xrpl_primitives;
        assert!(exists<TrustLineRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<TrustLineRegistry>(registry_addr);
        let id = generate_id(addr, counterparty);

        assert!(table::contains(&registry.lines, id), error::not_found(E_TRUST_LINE_NOT_FOUND));

        let trust_line = table::borrow_mut(&mut registry.lines, id);
        assert!(trust_line.balance == 0, error::invalid_state(E_INSUFFICIENT_LIMIT));

        trust_line.active = false;

        event::emit(TrustLineClosed {
            id,
            account1: trust_line.account1,
            account2: trust_line.account2,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Generate deterministic ID for a trust line pair
    fun generate_id(addr1: address, addr2: address): u128 {
        let (a1, a2) = if (addr1 < addr2) { (addr1, addr2) } else { (addr2, addr1) };
        ((a1 as u128) << 64) | (a2 as u128)
    }

    /// View functions
    #[view]
    public fun get_trust_line(account1: address, account2: address): (u64, u64, u64, bool, bool) acquires TrustLineRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<TrustLineRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<TrustLineRegistry>(registry_addr);
        let id = generate_id(account1, account2);

        assert!(table::contains(&registry.lines, id), error::not_found(E_TRUST_LINE_NOT_FOUND));

        let trust_line = table::borrow(&registry.lines, id);
        (trust_line.limit1, trust_line.limit2, trust_line.balance, trust_line.is_negative, trust_line.active)
    }

    #[view]
    public fun trust_line_exists(account1: address, account2: address): bool acquires TrustLineRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<TrustLineRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<TrustLineRegistry>(registry_addr);
        let id = generate_id(account1, account2);
        table::contains(&registry.lines, id)
    }
}
