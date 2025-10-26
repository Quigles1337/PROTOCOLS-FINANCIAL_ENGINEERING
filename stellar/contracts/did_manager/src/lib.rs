#![no_std]

//! DIDManager - W3C Decentralized Identifier Management
//! Production-grade Soroban implementation

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    Address, BytesN, Env, String, Vec, vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DIDDocument {
    pub id: String,
    pub controller: Address,
    pub verification_methods: Vec<BytesN<32>>,
    pub authentication: Vec<BytesN<32>>,
    pub service_endpoints: Vec<String>,
    pub created: u64,
    pub updated: u64,
    pub deactivated: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    DID(String),
    ControllerDIDs(Address),
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    AlreadyExists = 3,
    Deactivated = 4,
    InvalidMethod = 5,
}

#[contract]
pub struct DIDManagerContract;

#[contractimpl]
impl DIDManagerContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn create_did(env: Env, did_id: String, verification_methods: Vec<BytesN<32>>) -> Result<(), Error> {
        let controller = env.invoker();
        controller.require_auth();

        if env.storage().persistent().has(&DataKey::DID(did_id.clone())) {
            return Err(Error::AlreadyExists);
        }

        let doc = DIDDocument {
            id: did_id.clone(),
            controller: controller.clone(),
            verification_methods,
            authentication: vec![&env],
            service_endpoints: vec![&env],
            created: env.ledger().timestamp(),
            updated: env.ledger().timestamp(),
            deactivated: false,
        };

        env.storage().persistent().set(&DataKey::DID(did_id.clone()), &doc);
        env.storage().persistent().extend_ttl(&DataKey::DID(did_id.clone()), 518400, 518400);

        let mut controller_dids: Vec<String> = env.storage().persistent().get(&DataKey::ControllerDIDs(controller.clone())).unwrap_or(vec![&env]);
        controller_dids.push_back(did_id.clone());
        env.storage().persistent().set(&DataKey::ControllerDIDs(controller.clone()), &controller_dids);

        env.events().publish((symbol_short!("created"), controller), did_id);
        Ok(())
    }

    pub fn add_verification_method(env: Env, did_id: String, method: BytesN<32>) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut doc: DIDDocument = env.storage().persistent().get(&DataKey::DID(did_id.clone())).ok_or(Error::NotFound)?;
        if caller != doc.controller { return Err(Error::Unauthorized); }
        if doc.deactivated { return Err(Error::Deactivated); }

        doc.verification_methods.push_back(method);
        doc.updated = env.ledger().timestamp();
        env.storage().persistent().set(&DataKey::DID(did_id.clone()), &doc);
        env.events().publish((symbol_short!("updated"), did_id), ());
        Ok(())
    }

    pub fn add_service_endpoint(env: Env, did_id: String, endpoint: String) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut doc: DIDDocument = env.storage().persistent().get(&DataKey::DID(did_id.clone())).ok_or(Error::NotFound)?;
        if caller != doc.controller { return Err(Error::Unauthorized); }
        if doc.deactivated { return Err(Error::Deactivated); }

        doc.service_endpoints.push_back(endpoint);
        doc.updated = env.ledger().timestamp();
        env.storage().persistent().set(&DataKey::DID(did_id.clone()), &doc);
        Ok(())
    }

    pub fn transfer_control(env: Env, did_id: String, new_controller: Address) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut doc: DIDDocument = env.storage().persistent().get(&DataKey::DID(did_id.clone())).ok_or(Error::NotFound)?;
        if caller != doc.controller { return Err(Error::Unauthorized); }
        if doc.deactivated { return Err(Error::Deactivated); }

        let old_controller = doc.controller.clone();
        doc.controller = new_controller.clone();
        doc.updated = env.ledger().timestamp();
        env.storage().persistent().set(&DataKey::DID(did_id.clone()), &doc);

        env.events().publish((symbol_short!("transfer"), did_id), (old_controller, new_controller));
        Ok(())
    }

    pub fn deactivate_did(env: Env, did_id: String) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut doc: DIDDocument = env.storage().persistent().get(&DataKey::DID(did_id.clone())).ok_or(Error::NotFound)?;
        if caller != doc.controller { return Err(Error::Unauthorized); }
        if doc.deactivated { return Err(Error::Deactivated); }

        doc.deactivated = true;
        doc.updated = env.ledger().timestamp();
        env.storage().persistent().set(&DataKey::DID(did_id.clone()), &doc);

        env.events().publish((symbol_short!("deactivate"), did_id), ());
        Ok(())
    }

    pub fn get_did_document(env: Env, did_id: String) -> Option<DIDDocument> {
        env.storage().persistent().get(&DataKey::DID(did_id))
    }

    pub fn get_controller_dids(env: Env, controller: Address) -> Vec<String> {
        env.storage().persistent().get(&DataKey::ControllerDIDs(controller)).unwrap_or(vec![&env])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

    #[test]
    fn test_create_did() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DIDManagerContract);
        let client = DIDManagerContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let did_id = String::from_str(&env, "did:stellar:12345");
        let methods = vec![&env, BytesN::from_array(&env, &[1u8; 32])];
        client.create_did(&did_id, &methods);

        let doc = client.get_did_document(&did_id).unwrap();
        assert_eq!(doc.deactivated, false);
        assert_eq!(doc.verification_methods.len(), 1);
    }

    #[test]
    fn test_transfer_control() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DIDManagerContract);
        let client = DIDManagerContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let did_id = String::from_str(&env, "did:stellar:67890");
        let methods = vec![&env, BytesN::from_array(&env, &[2u8; 32])];
        client.create_did(&did_id, &methods);

        let new_controller = Address::generate(&env);
        client.transfer_control(&did_id, &new_controller);

        let doc = client.get_did_document(&did_id).unwrap();
        assert_eq!(doc.controller, new_controller);
    }

    #[test]
    fn test_deactivate_did() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DIDManagerContract);
        let client = DIDManagerContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let did_id = String::from_str(&env, "did:stellar:99999");
        let methods = vec![&env, BytesN::from_array(&env, &[3u8; 32])];
        client.create_did(&did_id, &methods);

        client.deactivate_did(&did_id);

        let doc = client.get_did_document(&did_id).unwrap();
        assert_eq!(doc.deactivated, true);
    }
}
