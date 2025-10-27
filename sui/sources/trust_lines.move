module xrpl_primitives::trust_lines {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::table::{Self, Table};
    use sui::event;

    // Errors
    const ERR_INVALID_LIMIT: u64 = 1;
    const ERR_INSUFFICIENT_LIMIT: u64 = 2;
    const ERR_NOT_AUTHORIZED: u64 = 3;
    const ERR_SELF_TRUST_LINE: u64 = 4;
    const ERR_TRUST_LINE_EXISTS: u64 = 5;
    const ERR_TRUST_LINE_NOT_FOUND: u64 = 6;

    // Structs
    public struct TrustLine has store {
        account1: address,
        account2: address,
        limit1: u64,
        limit2: u64,
        balance: u64,
        is_negative: bool,
    }

    public struct TrustLineRegistry has key {
        id: UID,
        trust_lines: Table<vector<u8>, TrustLine>,
    }

    // Events
    public struct TrustLineCreated has copy, drop {
        account1: address,
        account2: address,
        limit: u64,
    }

    public struct PaymentRippled has copy, drop {
        sender: address,
        receiver: address,
        amount: u64,
    }

    // Initialize the registry
    fun init(ctx: &mut TxContext) {
        let registry = TrustLineRegistry {
            id: object::new(ctx),
            trust_lines: table::new(ctx),
        };
        transfer::share_object(registry);
    }

    // Helper to generate key
    fun generate_key(addr1: address, addr2: address): vector<u8> {
        let key = vector::empty<u8>();
        let bytes1 = bcs::to_bytes(&addr1);
        let bytes2 = bcs::to_bytes(&addr2);

        if (addr1 < addr2) {
            vector::append(&mut key, bytes1);
            vector::append(&mut key, bytes2);
        } else {
            vector::append(&mut key, bytes2);
            vector::append(&mut key, bytes1);
        };
        key
    }

    // Create trust line
    public entry fun create_trust_line(
        registry: &mut TrustLineRegistry,
        counterparty: address,
        limit: u64,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(sender != counterparty, ERR_SELF_TRUST_LINE);
        assert!(limit > 0, ERR_INVALID_LIMIT);

        let key = generate_key(sender, counterparty);
        assert!(!table::contains(&registry.trust_lines, key), ERR_TRUST_LINE_EXISTS);

        let (account1, account2, limit1, limit2) = if (sender < counterparty) {
            (sender, counterparty, limit, 0)
        } else {
            (counterparty, sender, 0, limit)
        };

        let trust_line = TrustLine {
            account1,
            account2,
            limit1,
            limit2,
            balance: 0,
            is_negative: false,
        };

        table::add(&mut registry.trust_lines, key, trust_line);

        event::emit(TrustLineCreated {
            account1: sender,
            account2: counterparty,
            limit,
        });
    }

    // Update trust line limit
    public entry fun update_limit(
        registry: &mut TrustLineRegistry,
        counterparty: address,
        new_limit: u64,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(new_limit > 0, ERR_INVALID_LIMIT);

        let key = generate_key(sender, counterparty);
        assert!(table::contains(&registry.trust_lines, key), ERR_TRUST_LINE_NOT_FOUND);

        let trust_line = table::borrow_mut(&mut registry.trust_lines, key);

        if (sender == trust_line.account1) {
            trust_line.limit1 = new_limit;
        } else {
            trust_line.limit2 = new_limit;
        };
    }

    // Ripple payment
    public entry fun ripple_payment(
        registry: &mut TrustLineRegistry,
        receiver: address,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(amount > 0, ERR_INVALID_LIMIT);

        let key = generate_key(sender, receiver);
        assert!(table::contains(&registry.trust_lines, key), ERR_TRUST_LINE_NOT_FOUND);

        let trust_line = table::borrow_mut(&mut registry.trust_lines, key);
        let is_account1 = sender == trust_line.account1;

        if (is_account1) {
            if (trust_line.is_negative) {
                if (amount >= trust_line.balance) {
                    trust_line.balance = amount - trust_line.balance;
                    trust_line.is_negative = false;
                } else {
                    trust_line.balance = trust_line.balance - amount;
                };
            } else {
                let new_balance = trust_line.balance + amount;
                assert!(new_balance <= trust_line.limit2, ERR_INSUFFICIENT_LIMIT);
                trust_line.balance = new_balance;
            };
        } else {
            if (!trust_line.is_negative) {
                if (amount >= trust_line.balance) {
                    trust_line.balance = amount - trust_line.balance;
                    trust_line.is_negative = true;
                } else {
                    trust_line.balance = trust_line.balance - amount;
                };
            } else {
                let new_balance = trust_line.balance + amount;
                assert!(new_balance <= trust_line.limit1, ERR_INSUFFICIENT_LIMIT);
                trust_line.balance = new_balance;
            };
        };

        event::emit(PaymentRippled {
            sender,
            receiver,
            amount,
        });
    }

    // View functions
    public fun get_balance(
        registry: &TrustLineRegistry,
        account1: address,
        account2: address
    ): (u64, bool) {
        let key = generate_key(account1, account2);
        if (table::contains(&registry.trust_lines, key)) {
            let trust_line = table::borrow(&registry.trust_lines, key);
            (trust_line.balance, trust_line.is_negative)
        } else {
            (0, false)
        }
    }

    public fun get_available_credit(
        registry: &TrustLineRegistry,
        sender: address,
        receiver: address
    ): u64 {
        let key = generate_key(sender, receiver);
        if (!table::contains(&registry.trust_lines, key)) {
            return 0
        };

        let trust_line = table::borrow(&registry.trust_lines, key);
        let is_account1 = sender == trust_line.account1;

        if (is_account1) {
            if (trust_line.is_negative) {
                trust_line.balance
            } else {
                trust_line.limit2 - trust_line.balance
            }
        } else {
            if (!trust_line.is_negative) {
                trust_line.balance
            } else {
                trust_line.limit1 - trust_line.balance
            }
        }
    }
}
