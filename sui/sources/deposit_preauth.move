module xrpl_primitives::deposit_preauth {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::table::{Self, Table};
    use sui::event;
    use std::string::String;

    // Errors
    const ERR_INVALID_AMOUNT: u64 = 1;
    const ERR_PREAUTH_NOT_FOUND: u64 = 2;
    const ERR_NOT_AUTHORIZED: u64 = 3;
    const ERR_NOT_ACTIVE: u64 = 4;
    const ERR_EXPIRED: u64 = 5;
    const ERR_AMOUNT_EXCEEDS_LIMIT: u64 = 6;
    const ERR_SELF_PREAUTH: u64 = 7;

    // Status
    const STATUS_ACTIVE: u8 = 0;
    const STATUS_USED: u8 = 1;
    const STATUS_REVOKED: u8 = 2;

    // Structs
    public struct Preauth has store {
        authorizer: address,
        authorized: address,
        asset: String,
        max_amount: u64,
        expiration: u64,
        status: u8,
        created_at: u64,
    }

    public struct PreauthRegistry has key {
        id: UID,
        preauths: Table<u64, Preauth>,
        next_id: u64,
    }

    // Events
    public struct PreauthCreated has copy, drop {
        preauth_id: u64,
        authorizer: address,
        authorized: address,
    }

    public struct PreauthUsed has copy, drop {
        preauth_id: u64,
        amount: u64,
    }

    public struct PreauthRevoked has copy, drop {
        preauth_id: u64,
    }

    // Initialize registry
    fun init(ctx: &mut TxContext) {
        let registry = PreauthRegistry {
            id: object::new(ctx),
            preauths: table::new(ctx),
            next_id: 0,
        };
        transfer::share_object(registry);
    }

    // Create preauth
    public entry fun create_preauth(
        registry: &mut PreauthRegistry,
        authorized: address,
        asset: String,
        max_amount: u64,
        expiration: u64,
        ctx: &mut TxContext
    ): u64 {
        let authorizer = tx_context::sender(ctx);
        assert!(authorizer != authorized, ERR_SELF_PREAUTH);
        assert!(max_amount > 0, ERR_INVALID_AMOUNT);
        assert!(expiration > tx_context::epoch(ctx), ERR_EXPIRED);

        let preauth_id = registry.next_id;
        registry.next_id = registry.next_id + 1;

        let preauth = Preauth {
            authorizer,
            authorized,
            asset,
            max_amount,
            expiration,
            status: STATUS_ACTIVE,
            created_at: tx_context::epoch(ctx),
        };

        table::add(&mut registry.preauths, preauth_id, preauth);

        event::emit(PreauthCreated {
            preauth_id,
            authorizer,
            authorized,
        });

        preauth_id
    }

    // Use preauth
    public entry fun use_preauth(
        registry: &mut PreauthRegistry,
        preauth_id: u64,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let authorized = tx_context::sender(ctx);
        assert!(table::contains(&registry.preauths, preauth_id), ERR_PREAUTH_NOT_FOUND);

        let preauth = table::borrow_mut(&mut registry.preauths, preauth_id);
        assert!(preauth.authorized == authorized, ERR_NOT_AUTHORIZED);
        assert!(preauth.status == STATUS_ACTIVE, ERR_NOT_ACTIVE);
        assert!(tx_context::epoch(ctx) < preauth.expiration, ERR_EXPIRED);
        assert!(amount <= preauth.max_amount, ERR_AMOUNT_EXCEEDS_LIMIT);

        preauth.status = STATUS_USED;

        event::emit(PreauthUsed {
            preauth_id,
            amount,
        });
    }

    // Revoke preauth
    public entry fun revoke_preauth(
        registry: &mut PreauthRegistry,
        preauth_id: u64,
        ctx: &mut TxContext
    ) {
        let authorizer = tx_context::sender(ctx);
        assert!(table::contains(&registry.preauths, preauth_id), ERR_PREAUTH_NOT_FOUND);

        let preauth = table::borrow_mut(&mut registry.preauths, preauth_id);
        assert!(preauth.authorizer == authorizer, ERR_NOT_AUTHORIZED);
        assert!(preauth.status == STATUS_ACTIVE, ERR_NOT_ACTIVE);

        preauth.status = STATUS_REVOKED;

        event::emit(PreauthRevoked {
            preauth_id,
        });
    }

    // View functions
    public fun is_valid(registry: &PreauthRegistry, preauth_id: u64, current_epoch: u64): bool {
        if (table::contains(&registry.preauths, preauth_id)) {
            let preauth = table::borrow(&registry.preauths, preauth_id);
            preauth.status == STATUS_ACTIVE && current_epoch < preauth.expiration
        } else {
            false
        }
    }

    public fun get_preauth(registry: &PreauthRegistry, preauth_id: u64): Option<Preauth> {
        if (table::contains(&registry.preauths, preauth_id)) {
            option::some(*table::borrow(&registry.preauths, preauth_id))
        } else {
            option::none()
        }
    }
}
