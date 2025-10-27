use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DIDDocument {
    pub did: String,
    pub owner: AccountId,
    pub document_uri: String,
    pub active: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DIDManager {
    dids: UnorderedMap<String, DIDDocument>,
    account_to_did: UnorderedMap<AccountId, String>,
}

#[near_bindgen]
impl DIDManager {
    #[init]
    pub fn new() -> Self {
        Self {
            dids: UnorderedMap::new(b"d"),
            account_to_did: UnorderedMap::new(b"a"),
        }
    }

    pub fn register_did(&mut self, did: String, document_uri: String) {
        let owner = env::predecessor_account_id();

        assert!(!did.is_empty(), "DID cannot be empty");
        assert!(!document_uri.is_empty(), "Document URI required");
        assert!(!self.dids.get(&did).is_some(), "DID already exists");
        assert!(
            !self.account_to_did.get(&owner).is_some(),
            "Account already has DID"
        );

        let timestamp = env::block_timestamp();

        let did_document = DIDDocument {
            did: did.clone(),
            owner: owner.clone(),
            document_uri,
            active: true,
            created_at: timestamp,
            updated_at: timestamp,
        };

        self.dids.insert(&did, &did_document);
        self.account_to_did.insert(&owner, &did);
    }

    pub fn update_did(&mut self, new_document_uri: String) {
        let owner = env::predecessor_account_id();

        let did = self
            .account_to_did
            .get(&owner)
            .expect("No DID registered");

        let mut did_document = self.dids.get(&did).expect("DID not found");
        assert_eq!(did_document.owner, owner, "Not authorized");
        assert!(did_document.active, "DID not active");
        assert!(!new_document_uri.is_empty(), "Document URI required");

        did_document.document_uri = new_document_uri;
        did_document.updated_at = env::block_timestamp();

        self.dids.insert(&did, &did_document);
    }

    pub fn revoke_did(&mut self) {
        let owner = env::predecessor_account_id();

        let did = self
            .account_to_did
            .get(&owner)
            .expect("No DID registered");

        let mut did_document = self.dids.get(&did).expect("DID not found");
        assert_eq!(did_document.owner, owner, "Not authorized");
        assert!(did_document.active, "DID already revoked");

        did_document.active = false;
        did_document.updated_at = env::block_timestamp();

        self.dids.insert(&did, &did_document);
    }

    pub fn get_did(&self, did: String) -> Option<DIDDocument> {
        self.dids.get(&did)
    }

    pub fn get_did_by_account(&self, account: AccountId) -> Option<DIDDocument> {
        if let Some(did) = self.account_to_did.get(&account) {
            self.dids.get(&did)
        } else {
            None
        }
    }

    pub fn resolve_did(&self, did: String) -> Option<String> {
        if let Some(did_document) = self.dids.get(&did) {
            if did_document.active {
                Some(did_document.document_uri)
            } else {
                None
            }
        } else {
            None
        }
    }
}
