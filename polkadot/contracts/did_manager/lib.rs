#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod did_manager {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct DIDManager {
        admin: AccountId,
        dids: Mapping<AccountId, DIDDocument>,
        did_counter: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct DIDDocument {
        pub controller: AccountId,
        pub public_key: [u8; 32],
        pub service_endpoints: Vec<[u8; 64]>,
        pub created_at: u64,
        pub updated_at: u64,
        pub active: bool,
    }

    #[ink(event)]
    pub struct DIDCreated {
        #[ink(topic)]
        controller: AccountId,
    }

    #[ink(event)]
    pub struct DIDUpdated {
        #[ink(topic)]
        controller: AccountId,
    }

    #[ink(event)]
    pub struct DIDDeactivated {
        #[ink(topic)]
        controller: AccountId,
    }

    impl DIDManager {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                dids: Mapping::new(),
                did_counter: 0,
            }
        }

        #[ink(message)]
        pub fn create_did(&mut self, public_key: [u8; 32]) {
            let controller = self.env().caller();
            let current_block = self.env().block_number();

            assert!(!self.dids.contains(controller), "DID already exists");

            let did_doc = DIDDocument {
                controller,
                public_key,
                service_endpoints: vec![],
                created_at: current_block,
                updated_at: current_block,
                active: true,
            };

            self.dids.insert(controller, &did_doc);
            self.did_counter += 1;

            self.env().emit_event(DIDCreated { controller });
        }

        #[ink(message)]
        pub fn update_did(&mut self, public_key: [u8; 32], service_endpoints: Vec<[u8; 64]>) {
            let controller = self.env().caller();
            let current_block = self.env().block_number();
            let mut did_doc = self.dids.get(controller).expect("DID not found");

            assert!(did_doc.active, "DID not active");

            did_doc.public_key = public_key;
            did_doc.service_endpoints = service_endpoints;
            did_doc.updated_at = current_block;

            self.dids.insert(controller, &did_doc);

            self.env().emit_event(DIDUpdated { controller });
        }

        #[ink(message)]
        pub fn add_service_endpoint(&mut self, endpoint: [u8; 64]) {
            let controller = self.env().caller();
            let current_block = self.env().block_number();
            let mut did_doc = self.dids.get(controller).expect("DID not found");

            assert!(did_doc.active, "DID not active");

            did_doc.service_endpoints.push(endpoint);
            did_doc.updated_at = current_block;

            self.dids.insert(controller, &did_doc);
        }

        #[ink(message)]
        pub fn deactivate_did(&mut self) {
            let controller = self.env().caller();
            let mut did_doc = self.dids.get(controller).expect("DID not found");

            assert!(did_doc.active, "DID already deactivated");

            did_doc.active = false;
            self.dids.insert(controller, &did_doc);

            self.env().emit_event(DIDDeactivated { controller });
        }

        #[ink(message)]
        pub fn resolve_did(&self, controller: AccountId) -> Option<DIDDocument> {
            self.dids.get(controller)
        }

        #[ink(message)]
        pub fn verify_ownership(&self, controller: AccountId) -> bool {
            if let Some(did_doc) = self.dids.get(controller) {
                did_doc.active && did_doc.controller == controller
            } else {
                false
            }
        }

        #[ink(message)]
        pub fn get_did_count(&self) -> u64 {
            self.did_counter
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_create_did() {
            let mut contract = DIDManager::new();
            let public_key = [1u8; 32];
            contract.create_did(public_key);
            assert_eq!(contract.get_did_count(), 1);
        }
    }
}
