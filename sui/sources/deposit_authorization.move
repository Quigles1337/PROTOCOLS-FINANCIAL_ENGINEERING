module xrpl_primitives::deposit_authorization {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::table::{Self, Table};
    use sui::event;
    use std::string::String;

    // Errors
    const ERR_INVALID_AMOUNT: u64 = 1;
    const ERR_AUTH_EXISTS: u64 = 2;
    const ERR_AUTH_NOT_FOUND: u64 = 3;
    const ERR_NOT_AUTHORIZED: u64 = 4;
    const ERR_NOT_ACTIVE: u64 = 5;
    const ERR_EXPIRED: u64 = 6;
    const ERR_AMOUNT_EXCEEDS_LIMIT: u64 = 7;
    const ERR_EXCEEDS_TIER_LIMIT: u64 = 8;
    const ERR_SELF_AUTHORIZE: u64 = 9;

    // KYC Tiers
    const TIER_BASIC: u8 = 0;
    const TIER_STANDARD: u8 = 1;
    const TIER_PREMIUM: u8 = 2;
    const TIER_INSTITUTIONAL: u8 = 3;

    // Structs
    public struct Authorization has store {
        authorizer: address,
        authorized: address,
        asset: String,
        max_amount: u64,
        used_amount: u64,
        expiration: u64,
        tier: u8,
        active: bool,
        created_at: u64,
    }

    public struct AuthRegistry has key {
        id: UID,
        authorizations: Table<vector<u8>, Authorization>,
    }

    // Events
    public struct AuthorizationCreated has copy, drop {
        authorizer: address,
        authorized: address,
        max_amount: u64,
    }

    public struct AuthorizationUsed has copy, drop {
        authorizer: address,
        authorized: address,
        amount: u64,
    }

    public struct AuthorizationRevoked has copy, drop {
        authorizer: address,
        authorized: address,
    }

    // Initialize registry
    fun init(ctx: &mut TxContext) {
        let registry = AuthRegistry {
            id: object::new(ctx),
            authorizations: table::new(ctx),
        };
        transfer::share_object(registry);
    }

    // Helper functions
    fun generate_key(authorizer: address, authorized: address, asset: &String): vector<u8> {
        let key = vector::empty<u8>();
        vector::append(&mut key, bcs::to_bytes(&authorizer));
        vector::append(&mut key, bcs::to_bytes(&authorized));
        vector::append(&mut key, bcs::to_bytes(asset));
        key
    }

    fun get_tier_max_amount(tier: u8): u64 {
        if (tier == TIER_BASIC) { 1000000000 }        // 1 SUI
        else if (tier == TIER_STANDARD) { 10000000000 }    // 10 SUI
        else if (tier == TIER_PREMIUM) { 100000000000 }    // 100 SUI
        else { 1000000000000 }  // 1000 SUI (INSTITUTIONAL)
    }

    // Create authorization
    public entry fun create_authorization(
        registry: &mut AuthRegistry,
        authorized: address,
        asset: String,
        max_amount: u64,
        expiration: u64,
        tier: u8,
        ctx: &mut TxContext
    ) {
        let authorizer = tx_context::sender(ctx);
        assert!(authorizer != authorized, ERR_SELF_AUTHORIZE);
        assert!(max_amount > 0, ERR_INVALID_AMOUNT);
        assert!(expiration > tx_context::epoch(ctx), ERR_EXPIRED);
        assert!(max_amount <= get_tier_max_amount(tier), ERR_EXCEEDS_TIER_LIMIT);

        let key = generate_key(authorizer, authorized, &asset);
        assert!(!table::contains(&registry.authorizations, key), ERR_AUTH_EXISTS);

        let authorization = Authorization {
            authorizer,
            authorized,
            asset,
            max_amount,
            used_amount: 0,
            expiration,
            tier,
            active: true,
            created_at: tx_context::epoch(ctx),
        };

        table::add(&mut registry.authorizations, key, authorization);

        event::emit(AuthorizationCreated {
            authorizer,
            authorized,
            max_amount,
        });
    }

    // Use authorization
    public entry fun use_authorization(
        registry: &mut AuthRegistry,
        authorizer: address,
        asset: String,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let authorized = tx_context::sender(ctx);
        let key = generate_key(authorizer, authorized, &asset);
        assert!(table::contains(&registry.authorizations, key), ERR_AUTH_NOT_FOUND);

        let auth = table::borrow_mut(&mut registry.authorizations, key);
        assert!(auth.active, ERR_NOT_ACTIVE);
        assert!(tx_context::epoch(ctx) < auth.expiration, ERR_EXPIRED);
        assert!(auth.used_amount + amount <= auth.max_amount, ERR_AMOUNT_EXCEEDS_LIMIT);

        auth.used_amount = auth.used_amount + amount;

        event::emit(AuthorizationUsed {
            authorizer,
            authorized,
            amount,
        });
    }

    // Revoke authorization
    public entry fun revoke_authorization(
        registry: &mut AuthRegistry,
        authorized: address,
        asset: String,
        ctx: &mut TxContext
    ) {
        let authorizer = tx_context::sender(ctx);
        let key = generate_key(authorizer, authorized, &asset);
        assert!(table::contains(&registry.authorizations, key), ERR_AUTH_NOT_FOUND);

        let auth = table::borrow_mut(&mut registry.authorizations, key);
        assert!(auth.authorizer == authorizer, ERR_NOT_AUTHORIZED);
        assert!(auth.active, ERR_NOT_ACTIVE);

        auth.active = false;

        event::emit(AuthorizationRevoked {
            authorizer,
            authorized,
        });
    }

    // Update tier
    public entry fun update_tier(
        registry: &mut AuthRegistry,
        authorized: address,
        asset: String,
        new_tier: u8,
        ctx: &mut TxContext
    ) {
        let authorizer = tx_context::sender(ctx);
        let key = generate_key(authorizer, authorized, &asset);
        assert!(table::contains(&registry.authorizations, key), ERR_AUTH_NOT_FOUND);

        let auth = table::borrow_mut(&mut registry.authorizations, key);
        assert!(auth.authorizer == authorizer, ERR_NOT_AUTHORIZED);
        assert!(auth.active, ERR_NOT_ACTIVE);
        assert!(auth.max_amount <= get_tier_max_amount(new_tier), ERR_EXCEEDS_TIER_LIMIT);

        auth.tier = new_tier;
    }

    // View functions
    public fun get_remaining_amount(
        registry: &AuthRegistry,
        authorizer: address,
        authorized: address,
        asset: String
    ): u64 {
        let key = generate_key(authorizer, authorized, &asset);
        if (table::contains(&registry.authorizations, key)) {
            let auth = table::borrow(&registry.authorizations, key);
            auth.max_amount - auth.used_amount
        } else {
            0
        }
    }
}
