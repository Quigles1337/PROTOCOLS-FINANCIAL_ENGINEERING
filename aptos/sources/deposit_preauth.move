module xrpl_primitives::deposit_preauth {
    use std::signer;
    use std::error;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info::{Self, TypeInfo};

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_PREAUTH_NOT_FOUND: u64 = 3;
    const E_UNAUTHORIZED: u64 = 4;
    const E_ALREADY_USED: u64 = 5;
    const E_EXPIRED: u64 = 6;
    const E_AMOUNT_EXCEEDS_MAX: u64 = 7;

    /// Preauthorization for one-time deposits
    struct Preauth has store, copy, drop {
        authorizer: address,
        authorized: address,
        asset: TypeInfo,
        max_amount: u64,
        expiration: u64,
        used: bool,
        created_at: u64,
    }

    /// Global preauth registry
    struct PreauthRegistry has key {
        preauths: Table<u64, Preauth>,
        next_id: u64,
    }

    /// Events
    #[event]
    struct PreauthCreated has drop, store {
        preauth_id: u64,
        authorizer: address,
        authorized: address,
        max_amount: u64,
        expiration: u64,
        timestamp: u64,
    }

    #[event]
    struct PreauthUsed has drop, store {
        preauth_id: u64,
        authorized: address,
        amount: u64,
        timestamp: u64,
    }

    #[event]
    struct PreauthRevoked has drop, store {
        preauth_id: u64,
        authorizer: address,
        timestamp: u64,
    }

    /// Initialize the preauth registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<PreauthRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, PreauthRegistry {
            preauths: table::new(),
            next_id: 0,
        });
    }

    /// Create a preauthorization
    public fun create_preauth<Asset>(
        authorizer: &signer,
        authorized: address,
        max_amount: u64,
        expiration: u64,
    ): u64 acquires PreauthRegistry {
        let authorizer_addr = signer::address_of(authorizer);

        let registry_addr = @xrpl_primitives;
        assert!(exists<PreauthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let now = timestamp::now_seconds();
        assert!(expiration > now, error::invalid_argument(E_EXPIRED));

        let registry = borrow_global_mut<PreauthRegistry>(registry_addr);
        let preauth_id = registry.next_id;
        registry.next_id = preauth_id + 1;

        let asset = type_info::type_of<Asset>();

        let preauth = Preauth {
            authorizer: authorizer_addr,
            authorized,
            asset,
            max_amount,
            expiration,
            used: false,
            created_at: now,
        };

        table::add(&mut registry.preauths, preauth_id, preauth);

        event::emit(PreauthCreated {
            preauth_id,
            authorizer: authorizer_addr,
            authorized,
            max_amount,
            expiration,
            timestamp: now,
        });

        preauth_id
    }

    /// Use a preauthorization (single use only)
    public fun use_preauth<Asset>(
        authorized: &signer,
        preauth_id: u64,
        amount: u64,
    ) acquires PreauthRegistry {
        let authorized_addr = signer::address_of(authorized);

        let registry_addr = @xrpl_primitives;
        assert!(exists<PreauthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<PreauthRegistry>(registry_addr);
        assert!(table::contains(&registry.preauths, preauth_id), error::not_found(E_PREAUTH_NOT_FOUND));

        let preauth = table::borrow_mut(&mut registry.preauths, preauth_id);
        assert!(preauth.authorized == authorized_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(!preauth.used, error::invalid_state(E_ALREADY_USED));

        let now = timestamp::now_seconds();
        assert!(now < preauth.expiration, error::invalid_state(E_EXPIRED));
        assert!(amount <= preauth.max_amount, error::invalid_argument(E_AMOUNT_EXCEEDS_MAX));

        // Verify asset type matches
        let asset = type_info::type_of<Asset>();
        assert!(asset == preauth.asset, error::invalid_argument(E_UNAUTHORIZED));

        // Mark as used (single-use token)
        preauth.used = true;

        event::emit(PreauthUsed {
            preauth_id,
            authorized: authorized_addr,
            amount,
            timestamp: now,
        });
    }

    /// Revoke a preauthorization
    public entry fun revoke_preauth(
        authorizer: &signer,
        preauth_id: u64,
    ) acquires PreauthRegistry {
        let authorizer_addr = signer::address_of(authorizer);

        let registry_addr = @xrpl_primitives;
        assert!(exists<PreauthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<PreauthRegistry>(registry_addr);
        assert!(table::contains(&registry.preauths, preauth_id), error::not_found(E_PREAUTH_NOT_FOUND));

        let preauth = table::borrow_mut(&mut registry.preauths, preauth_id);
        assert!(preauth.authorizer == authorizer_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(!preauth.used, error::invalid_state(E_ALREADY_USED));

        // Mark as used to prevent future use
        preauth.used = true;

        event::emit(PreauthRevoked {
            preauth_id,
            authorizer: authorizer_addr,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// View functions
    #[view]
    public fun get_preauth(preauth_id: u64): (address, address, u64, u64, bool) acquires PreauthRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<PreauthRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<PreauthRegistry>(registry_addr);
        assert!(table::contains(&registry.preauths, preauth_id), error::not_found(E_PREAUTH_NOT_FOUND));

        let preauth = table::borrow(&registry.preauths, preauth_id);
        (preauth.authorizer, preauth.authorized, preauth.max_amount, preauth.expiration, preauth.used)
    }

    #[view]
    public fun is_valid(preauth_id: u64): bool acquires PreauthRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<PreauthRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<PreauthRegistry>(registry_addr);
        if (!table::contains(&registry.preauths, preauth_id)) {
            return false
        };

        let preauth = table::borrow(&registry.preauths, preauth_id);
        let now = timestamp::now_seconds();
        !preauth.used && now < preauth.expiration
    }

    #[view]
    public fun is_expired(preauth_id: u64): bool acquires PreauthRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<PreauthRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<PreauthRegistry>(registry_addr);
        if (!table::contains(&registry.preauths, preauth_id)) {
            return false
        };

        let preauth = table::borrow(&registry.preauths, preauth_id);
        timestamp::now_seconds() >= preauth.expiration
    }
}
