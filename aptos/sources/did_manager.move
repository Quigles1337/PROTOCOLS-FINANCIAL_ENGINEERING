module xrpl_primitives::did_manager {
    use std::signer;
    use std::error;
    use std::string::{Self, String};
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_DID_NOT_FOUND: u64 = 3;
    const E_UNAUTHORIZED: u64 = 4;
    const E_DID_EXISTS: u64 = 5;
    const E_INVALID_DID: u64 = 6;
    const E_DID_REVOKED: u64 = 7;

    /// DID document representing a decentralized identifier
    struct DIDDocument has store, copy, drop {
        owner: address,
        did: String,
        document_uri: String,
        active: bool,
        created_at: u64,
        updated_at: u64,
    }

    /// Global DID registry
    struct DIDRegistry has key {
        dids: Table<address, DIDDocument>,
        did_to_address: Table<String, address>,
    }

    /// Events
    #[event]
    struct DIDCreated has drop, store {
        owner: address,
        did: String,
        document_uri: String,
        timestamp: u64,
    }

    #[event]
    struct DIDUpdated has drop, store {
        owner: address,
        did: String,
        new_document_uri: String,
        timestamp: u64,
    }

    #[event]
    struct DIDRevoked has drop, store {
        owner: address,
        did: String,
        timestamp: u64,
    }

    /// Initialize the DID registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<DIDRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, DIDRegistry {
            dids: table::new(),
            did_to_address: table::new(),
        });
    }

    /// Register a new DID
    public entry fun register_did(
        owner: &signer,
        did: String,
        document_uri: String,
    ) acquires DIDRegistry {
        let owner_addr = signer::address_of(owner);
        assert!(string::length(&did) > 0, error::invalid_argument(E_INVALID_DID));

        let registry_addr = @xrpl_primitives;
        assert!(exists<DIDRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<DIDRegistry>(registry_addr);
        assert!(!table::contains(&registry.dids, owner_addr), error::already_exists(E_DID_EXISTS));
        assert!(!table::contains(&registry.did_to_address, did), error::already_exists(E_DID_EXISTS));

        let now = timestamp::now_seconds();

        let did_doc = DIDDocument {
            owner: owner_addr,
            did,
            document_uri,
            active: true,
            created_at: now,
            updated_at: now,
        };

        table::add(&mut registry.dids, owner_addr, did_doc);
        table::add(&mut registry.did_to_address, did, owner_addr);

        event::emit(DIDCreated {
            owner: owner_addr,
            did,
            document_uri,
            timestamp: now,
        });
    }

    /// Update DID document
    public entry fun update_did(
        owner: &signer,
        new_document_uri: String,
    ) acquires DIDRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<DIDRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<DIDRegistry>(registry_addr);
        assert!(table::contains(&registry.dids, owner_addr), error::not_found(E_DID_NOT_FOUND));

        let did_doc = table::borrow_mut(&mut registry.dids, owner_addr);
        assert!(did_doc.active, error::invalid_state(E_DID_REVOKED));

        did_doc.document_uri = new_document_uri;
        did_doc.updated_at = timestamp::now_seconds();

        event::emit(DIDUpdated {
            owner: owner_addr,
            did: did_doc.did,
            new_document_uri,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Revoke a DID
    public entry fun revoke_did(
        owner: &signer,
    ) acquires DIDRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<DIDRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<DIDRegistry>(registry_addr);
        assert!(table::contains(&registry.dids, owner_addr), error::not_found(E_DID_NOT_FOUND));

        let did_doc = table::borrow_mut(&mut registry.dids, owner_addr);
        assert!(did_doc.active, error::invalid_state(E_DID_REVOKED));

        did_doc.active = false;
        did_doc.updated_at = timestamp::now_seconds();

        event::emit(DIDRevoked {
            owner: owner_addr,
            did: did_doc.did,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// View functions
    #[view]
    public fun get_did_by_address(owner: address): (String, String, bool, u64, u64) acquires DIDRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<DIDRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<DIDRegistry>(registry_addr);
        assert!(table::contains(&registry.dids, owner), error::not_found(E_DID_NOT_FOUND));

        let did_doc = table::borrow(&registry.dids, owner);
        (did_doc.did, did_doc.document_uri, did_doc.active, did_doc.created_at, did_doc.updated_at)
    }

    #[view]
    public fun get_address_by_did(did: String): address acquires DIDRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<DIDRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<DIDRegistry>(registry_addr);
        assert!(table::contains(&registry.did_to_address, did), error::not_found(E_DID_NOT_FOUND));

        *table::borrow(&registry.did_to_address, did)
    }

    #[view]
    public fun has_did(owner: address): bool acquires DIDRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<DIDRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<DIDRegistry>(registry_addr);
        table::contains(&registry.dids, owner)
    }

    #[view]
    public fun is_did_active(owner: address): bool acquires DIDRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<DIDRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<DIDRegistry>(registry_addr);
        if (!table::contains(&registry.dids, owner)) {
            return false
        };

        let did_doc = table::borrow(&registry.dids, owner);
        did_doc.active
    }
}
