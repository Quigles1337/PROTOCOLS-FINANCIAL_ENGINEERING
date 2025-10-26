#![no_std]

//! DepositAuthorization - KYC/AML Compliance Whitelisting
//! Production-grade Soroban implementation

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    Address, Env, Vec, vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizationStatus {
    Authorized,
    Revoked,
    Pending,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Authorization {
    pub authorizer: Address,
    pub authorized_account: Address,
    pub token: Address,
    pub status: AuthorizationStatus,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Authorization(Address, Address, Address),
    AuthorizedAccounts(Address, Address),
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    AlreadyAuthorized = 3,
    NotAuthorized = 4,
    Expired = 5,
}

#[contract]
pub struct DepositAuthorizationContract;

#[contractimpl]
impl DepositAuthorizationContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn authorize_account(env: Env, account: Address, token: Address, duration: Option<u32>) -> Result<(), Error> {
        let authorizer = env.invoker();
        authorizer.require_auth();

        let key = DataKey::Authorization(authorizer.clone(), account.clone(), token.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadyAuthorized);
        }

        let expires_at = duration.map(|d| env.ledger().timestamp() + (d as u64));

        let auth = Authorization {
            authorizer: authorizer.clone(),
            authorized_account: account.clone(),
            token: token.clone(),
            status: AuthorizationStatus::Authorized,
            created_at: env.ledger().timestamp(),
            expires_at,
        };

        env.storage().persistent().set(&key, &auth);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        let accounts_key = DataKey::AuthorizedAccounts(authorizer.clone(), token.clone());
        let mut accounts: Vec<Address> = env.storage().persistent().get(&accounts_key).unwrap_or(vec![&env]);
        accounts.push_back(account.clone());
        env.storage().persistent().set(&accounts_key, &accounts);

        env.events().publish((symbol_short!("authorize"), authorizer, account), token);
        Ok(())
    }

    pub fn revoke_authorization(env: Env, account: Address, token: Address) -> Result<(), Error> {
        let authorizer = env.invoker();
        authorizer.require_auth();

        let key = DataKey::Authorization(authorizer.clone(), account.clone(), token.clone());
        let mut auth: Authorization = env.storage().persistent().get(&key).ok_or(Error::NotFound)?;

        auth.status = AuthorizationStatus::Revoked;
        env.storage().persistent().set(&key, &auth);

        env.events().publish((symbol_short!("revoke"), authorizer, account), token);
        Ok(())
    }

    pub fn is_authorized(env: Env, authorizer: Address, account: Address, token: Address) -> Result<bool, Error> {
        let key = DataKey::Authorization(authorizer, account, token);
        let auth: Authorization = env.storage().persistent().get(&key).ok_or(Error::NotFound)?;

        if !matches!(auth.status, AuthorizationStatus::Authorized) {
            return Ok(false);
        }

        if let Some(expires) = auth.expires_at {
            if env.ledger().timestamp() >= expires {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn get_authorization(env: Env, authorizer: Address, account: Address, token: Address) -> Option<Authorization> {
        let key = DataKey::Authorization(authorizer, account, token);
        env.storage().persistent().get(&key)
    }

    pub fn get_authorized_accounts(env: Env, authorizer: Address, token: Address) -> Vec<Address> {
        let key = DataKey::AuthorizedAccounts(authorizer, token);
        env.storage().persistent().get(&key).unwrap_or(vec![&env])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_authorize_account() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DepositAuthorizationContract);
        let client = DepositAuthorizationContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let authorizer = Address::generate(&env);
        let account = Address::generate(&env);
        let token = Address::generate(&env);

        client.authorize_account(&account, &token, &None);

        let is_auth = client.is_authorized(&authorizer, &account, &token);
        assert_eq!(is_auth, true);
    }

    #[test]
    fn test_revoke_authorization() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DepositAuthorizationContract);
        let client = DepositAuthorizationContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let authorizer = Address::generate(&env);
        let account = Address::generate(&env);
        let token = Address::generate(&env);

        client.authorize_account(&account, &token, &None);
        client.revoke_authorization(&account, &token);

        let is_auth = client.is_authorized(&authorizer, &account, &token);
        assert_eq!(is_auth, false);
    }
}
