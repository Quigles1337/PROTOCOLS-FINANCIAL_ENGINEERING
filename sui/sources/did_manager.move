module xrpl_primitives::did_manager {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::table::{Self, Table};
    use sui::event;
    use std::string::String;

    // Errors
    const ERR_DID_EXISTS: u64 = 1;
    const ERR_DID_NOT_FOUND: u64 = 2;
    const ERR_NOT_AUTHORIZED: u64 = 3;
    const ERR_NOT_ACTIVE: u64 = 4;
    const ERR_ACCOUNT_HAS_DID: u64 = 5;

    // Structs
    public struct DIDDocument has store {
        did: String,
        owner: address,
        document_uri: String,
        active: bool,
        created_at: u64,
        updated_at: u64,
    }

    public struct DIDRegistry has key {
        id: UID,
        dids: Table<String, DIDDocument>,
        account_to_did: Table<address, String>,
    }

    // Events
    public struct DIDRegistered has copy, drop {
        did: String,
        owner: address,
    }

    public struct DIDUpdated has copy, drop {
        did: String,
    }

    public struct DIDRevoked has copy, drop {
        did: String,
    }

    // Initialize registry
    fun init(ctx: &mut TxContext) {
        let registry = DIDRegistry {
            id: object::new(ctx),
            dids: table::new(ctx),
            account_to_did: table::new(ctx),
        };
        transfer::share_object(registry);
    }

    // Register DID
    public entry fun register_did(
        registry: &mut DIDRegistry,
        did: String,
        document_uri: String,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(!table::contains(&registry.dids, did), ERR_DID_EXISTS);
        assert!(!table::contains(&registry.account_to_did, owner), ERR_ACCOUNT_HAS_DID);

        let timestamp = tx_context::epoch(ctx);

        let did_document = DIDDocument {
            did,
            owner,
            document_uri,
            active: true,
            created_at: timestamp,
            updated_at: timestamp,
        };

        table::add(&mut registry.dids, did, did_document);
        table::add(&mut registry.account_to_did, owner, did);

        event::emit(DIDRegistered {
            did,
            owner,
        });
    }

    // Update DID
    public entry fun update_did(
        registry: &mut DIDRegistry,
        new_document_uri: String,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(table::contains(&registry.account_to_did, owner), ERR_DID_NOT_FOUND);

        let did = *table::borrow(&registry.account_to_did, owner);
        let did_document = table::borrow_mut(&mut registry.dids, did);

        assert!(did_document.owner == owner, ERR_NOT_AUTHORIZED);
        assert!(did_document.active, ERR_NOT_ACTIVE);

        did_document.document_uri = new_document_uri;
        did_document.updated_at = tx_context::epoch(ctx);

        event::emit(DIDUpdated {
            did,
        });
    }

    // Revoke DID
    public entry fun revoke_did(
        registry: &mut DIDRegistry,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(table::contains(&registry.account_to_did, owner), ERR_DID_NOT_FOUND);

        let did = *table::borrow(&registry.account_to_did, owner);
        let did_document = table::borrow_mut(&mut registry.dids, did);

        assert!(did_document.owner == owner, ERR_NOT_AUTHORIZED);
        assert!(did_document.active, ERR_NOT_ACTIVE);

        did_document.active = false;
        did_document.updated_at = tx_context::epoch(ctx);

        event::emit(DIDRevoked {
            did,
        });
    }

    // View functions
    public fun resolve_did(registry: &DIDRegistry, did: String): Option<String> {
        if (table::contains(&registry.dids, did)) {
            let did_document = table::borrow(&registry.dids, did);
            if (did_document.active) {
                option::some(did_document.document_uri)
            } else {
                option::none()
            }
        } else {
            option::none()
        }
    }

    public fun get_did_by_account(registry: &DIDRegistry, account: address): Option<DIDDocument> {
        if (table::contains(&registry.account_to_did, account)) {
            let did = *table::borrow(&registry.account_to_did, account);
            if (table::contains(&registry.dids, did)) {
                option::some(*table::borrow(&registry.dids, did))
            } else {
                option::none()
            }
        } else {
            option::none()
        }
    }
}
