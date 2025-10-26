#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountInfo { pub owner: Address, pub created_at: u64, pub deleted: bool, pub deleted_at: Option<u64>, pub beneficiary: Option<Address> }

#[contracttype]
#[derive(Clone)]
pub enum DataKey { Account(Address), Admin }

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error { NotFound = 1, Unauthorized = 2, AlreadyDeleted = 3, TooYoung = 4 }

#[contract]
pub struct AccountDeleteContract;

#[contractimpl]
impl AccountDeleteContract {
    pub fn initialize(env: Env, admin: Address) { admin.require_auth(); env.storage().instance().set(&DataKey::Admin, &admin); }

    pub fn register_account(env: Env) -> Result<(), Error> {
        let owner = env.invoker(); owner.require_auth();
        let account = AccountInfo { owner: owner.clone(), created_at: env.ledger().timestamp(), deleted: false, deleted_at: None, beneficiary: None };
        env.storage().persistent().set(&DataKey::Account(owner.clone()), &account);
        env.storage().persistent().extend_ttl(&DataKey::Account(owner.clone()), 518400, 518400);
        env.events().publish((symbol_short!("registered"), owner), ());
        Ok(())
    }

    pub fn set_beneficiary(env: Env, beneficiary: Address) -> Result<(), Error> {
        let owner = env.invoker(); owner.require_auth();
        let mut account: AccountInfo = env.storage().persistent().get(&DataKey::Account(owner.clone())).ok_or(Error::NotFound)?;
        if account.deleted { return Err(Error::AlreadyDeleted); }
        account.beneficiary = Some(beneficiary.clone());
        env.storage().persistent().set(&DataKey::Account(owner.clone()), &account);
        env.events().publish((symbol_short!("beneficiary"), owner, beneficiary), ());
        Ok(())
    }

    pub fn delete_account(env: Env, tokens: soroban_sdk::Vec<Address>) -> Result<(), Error> {
        let owner = env.invoker(); owner.require_auth();
        let mut account: AccountInfo = env.storage().persistent().get(&DataKey::Account(owner.clone())).ok_or(Error::NotFound)?;
        if account.deleted { return Err(Error::AlreadyDeleted); }
        let age = env.ledger().timestamp() - account.created_at;
        if age < 86400 { return Err(Error::TooYoung); }
        
        let beneficiary = account.beneficiary.clone().unwrap_or(owner.clone());
        for token_addr in tokens.iter() {
            let token_client = token::Client::new(&env, &token_addr);
            let balance = token_client.balance(&owner);
            if balance > 0 { token_client.transfer(&owner, &beneficiary, &balance); }
        }
        
        account.deleted = true;
        account.deleted_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&DataKey::Account(owner.clone()), &account);
        env.events().publish((symbol_short!("deleted"), owner), beneficiary);
        Ok(())
    }

    pub fn get_account(env: Env, owner: Address) -> Option<AccountInfo> { env.storage().persistent().get(&DataKey::Account(owner)) }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, vec};

    #[test]
    fn test_register_and_delete() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AccountDeleteContract);
        let client = AccountDeleteContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let owner = Address::generate(&env);
        client.register_account();

        env.ledger().with_mut(|li| li.timestamp = 100000);

        let tokens = vec![&env];
        client.delete_account(&tokens);

        let account = client.get_account(&owner).unwrap();
        assert_eq!(account.deleted, true);
    }
}
