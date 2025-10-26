module xrpl_primitives::deposit_authorization {
    use std::signer;
    use std::error;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info::{Self, TypeInfo};

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_AUTH_NOT_FOUND: u64 = 3;
    const E_UNAUTHORIZED: u64 = 4;
    const E_AUTH_EXPIRED: u64 = 5;
    const E_INVALID_TIER: u64 = 6;
    const E_AMOUNT_EXCEEDS_LIMIT: u64 = 7;
    const E_AUTH_REVOKED: u64 = 8;

    /// Authorization tiers (KYC/AML levels)
    const TIER_BASIC: u8 = 1;
    const TIER_STANDARD: u8 = 2;
    const TIER_PREMIUM: u8 = 3;
    const TIER_INSTITUTIONAL: u8 = 4;

    /// Deposit authorization for KYC/AML compliance
    struct DepositAuth has store, copy, drop {
        authorizer: address,
        authorized: address,
        asset: TypeInfo,
        max_amount: u64,
        expiration: u64,
        tier: u8,
        active: bool,
        created_at: u64,
    }

    /// Global authorization registry
    struct AuthRegistry has key {
        authorizations: Table<u128, DepositAuth>, // key = hash(authorizer, authorized, asset)
    }

    /// Events
    #[event]
    struct AuthorizationCreated has drop, store {
        id: u128,
        authorizer: address,
        authorized: address,
        max_amount: u64,
        expiration: u64,
        tier: u8,
        timestamp: u64,
    }

    #[event]
    struct AuthorizationUsed has drop, store {
        id: u128,
        authorized: address,
        amount: u64,
        timestamp: u64,
    }

    #[event]
    struct AuthorizationRevoked has drop, store {
        id: u128,
        authorizer: address,
        timestamp: u64,
    }

    #[event]
    struct TierUpdated has drop, store {
        id: u128,
        new_tier: u8,
        timestamp: u64,
    }

    /// Initialize the authorization registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<AuthRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, AuthRegistry {
            authorizations: table::new(),
        });
    }

    /// Create a deposit authorization
    public fun create_authorization<Asset>(
        authorizer: &signer,
        authorized: address,
        max_amount: u64,
        expiration: u64,
        tier: u8,
    ) acquires AuthRegistry {
        let authorizer_addr = signer::address_of(authorizer);
        assert!(is_valid_tier(tier), error::invalid_argument(E_INVALID_TIER));

        let registry_addr = @xrpl_primitives;
        assert!(exists<AuthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let now = timestamp::now_seconds();
        assert!(expiration > now, error::invalid_argument(E_AUTH_EXPIRED));

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer_addr, authorized, asset);

        let registry = borrow_global_mut<AuthRegistry>(registry_addr);

        let auth = DepositAuth {
            authorizer: authorizer_addr,
            authorized,
            asset,
            max_amount,
            expiration,
            tier,
            active: true,
            created_at: now,
        };

        if (table::contains(&registry.authorizations, id)) {
            // Update existing authorization
            let existing = table::borrow_mut(&mut registry.authorizations, id);
            *existing = auth;
        } else {
            table::add(&mut registry.authorizations, id, auth);
        };

        event::emit(AuthorizationCreated {
            id,
            authorizer: authorizer_addr,
            authorized,
            max_amount,
            expiration,
            tier,
            timestamp: now,
        });
    }

    /// Check and validate authorization for deposit
    public fun validate_deposit<Asset>(
        authorized: address,
        authorizer: address,
        amount: u64,
    ): bool acquires AuthRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<AuthRegistry>(registry_addr)) {
            return false
        };

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer, authorized, asset);

        let registry = borrow_global<AuthRegistry>(registry_addr);
        if (!table::contains(&registry.authorizations, id)) {
            return false
        };

        let auth = table::borrow(&registry.authorizations, id);

        if (!auth.active) {
            return false
        };

        let now = timestamp::now_seconds();
        if (now >= auth.expiration) {
            return false
        };

        if (amount > auth.max_amount) {
            return false
        };

        true
    }

    /// Use authorization (called during deposit)
    public fun use_authorization<Asset>(
        authorized: &signer,
        authorizer: address,
        amount: u64,
    ) acquires AuthRegistry {
        let authorized_addr = signer::address_of(authorized);

        let registry_addr = @xrpl_primitives;
        assert!(exists<AuthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer, authorized_addr, asset);

        let registry = borrow_global<AuthRegistry>(registry_addr);
        assert!(table::contains(&registry.authorizations, id), error::not_found(E_AUTH_NOT_FOUND));

        let auth = table::borrow(&registry.authorizations, id);
        assert!(auth.active, error::invalid_state(E_AUTH_REVOKED));

        let now = timestamp::now_seconds();
        assert!(now < auth.expiration, error::invalid_state(E_AUTH_EXPIRED));
        assert!(amount <= auth.max_amount, error::invalid_argument(E_AMOUNT_EXCEEDS_LIMIT));

        event::emit(AuthorizationUsed {
            id,
            authorized: authorized_addr,
            amount,
            timestamp: now,
        });
    }

    /// Revoke authorization
    public entry fun revoke_authorization<Asset>(
        authorizer: &signer,
        authorized: address,
    ) acquires AuthRegistry {
        let authorizer_addr = signer::address_of(authorizer);

        let registry_addr = @xrpl_primitives;
        assert!(exists<AuthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer_addr, authorized, asset);

        let registry = borrow_global_mut<AuthRegistry>(registry_addr);
        assert!(table::contains(&registry.authorizations, id), error::not_found(E_AUTH_NOT_FOUND));

        let auth = table::borrow_mut(&mut registry.authorizations, id);
        assert!(auth.authorizer == authorizer_addr, error::permission_denied(E_UNAUTHORIZED));

        auth.active = false;

        event::emit(AuthorizationRevoked {
            id,
            authorizer: authorizer_addr,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Update tier
    public entry fun update_tier<Asset>(
        authorizer: &signer,
        authorized: address,
        new_tier: u8,
    ) acquires AuthRegistry {
        let authorizer_addr = signer::address_of(authorizer);
        assert!(is_valid_tier(new_tier), error::invalid_argument(E_INVALID_TIER));

        let registry_addr = @xrpl_primitives;
        assert!(exists<AuthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer_addr, authorized, asset);

        let registry = borrow_global_mut<AuthRegistry>(registry_addr);
        assert!(table::contains(&registry.authorizations, id), error::not_found(E_AUTH_NOT_FOUND));

        let auth = table::borrow_mut(&mut registry.authorizations, id);
        assert!(auth.authorizer == authorizer_addr, error::permission_denied(E_UNAUTHORIZED));

        auth.tier = new_tier;

        event::emit(TierUpdated {
            id,
            new_tier,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Helper functions
    fun generate_id(authorizer: address, authorized: address, asset: TypeInfo): u128 {
        let auth_hash = ((authorizer as u128) << 64) | (authorized as u128);
        // Simple hash combining addresses (in production, use proper hash function)
        auth_hash
    }

    fun is_valid_tier(tier: u8): bool {
        tier == TIER_BASIC || tier == TIER_STANDARD || tier == TIER_PREMIUM || tier == TIER_INSTITUTIONAL
    }

    /// View functions
    #[view]
    public fun get_authorization<Asset>(authorizer: address, authorized: address): (u64, u64, u8, bool) acquires AuthRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<AuthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer, authorized, asset);

        let registry = borrow_global<AuthRegistry>(registry_addr);
        assert!(table::contains(&registry.authorizations, id), error::not_found(E_AUTH_NOT_FOUND));

        let auth = table::borrow(&registry.authorizations, id);
        (auth.max_amount, auth.expiration, auth.tier, auth.active)
    }

    #[view]
    public fun is_authorized<Asset>(authorizer: address, authorized: address): bool acquires AuthRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<AuthRegistry>(registry_addr)) {
            return false
        };

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer, authorized, asset);

        let registry = borrow_global<AuthRegistry>(registry_addr);
        if (!table::contains(&registry.authorizations, id)) {
            return false
        };

        let auth = table::borrow(&registry.authorizations, id);
        let now = timestamp::now_seconds();
        auth.active && now < auth.expiration
    }

    #[view]
    public fun get_tier<Asset>(authorizer: address, authorized: address): u8 acquires AuthRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<AuthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let asset = type_info::type_of<Asset>();
        let id = generate_id(authorizer, authorized, asset);

        let registry = borrow_global<AuthRegistry>(registry_addr);
        assert!(table::contains(&registry.authorizations, id), error::not_found(E_AUTH_NOT_FOUND));

        let auth = table::borrow(&registry.authorizations, id);
        auth.tier
    }
}
