#![no_std]

//! DepositPreauth - One-Time Deposit Pre-Authorizations
//! Production-grade Soroban implementation

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    Address, BytesN, Env,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preauth {
    pub creator: Address,
    pub authorized_depositor: Address,
    pub token: Address,
    pub max_amount: Option<i128>,
    pub used: bool,
    pub created_at: u64,
    pub used_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Preauth(BytesN<32>),
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    AlreadyUsed = 3,
    ExceedsMaxAmount = 4,
}

#[contract]
pub struct DepositPreauthContract;

#[contractimpl]
impl DepositPreauthContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn create_preauth(env: Env, depositor: Address, token: Address, max_amount: Option<i128>) -> BytesN<32> {
        let creator = env.invoker();
        creator.require_auth();

        let preauth_id = env.crypto().sha256(&(creator.clone(), depositor.clone(), token.clone(), env.ledger().timestamp()).try_into_val(&env).unwrap());

        let preauth = Preauth {
            creator: creator.clone(),
            authorized_depositor: depositor.clone(),
            token: token.clone(),
            max_amount,
            used: false,
            created_at: env.ledger().timestamp(),
            used_at: None,
        };

        env.storage().persistent().set(&DataKey::Preauth(preauth_id.clone()), &preauth);
        env.storage().persistent().extend_ttl(&DataKey::Preauth(preauth_id.clone()), 518400, 518400);

        env.events().publish((symbol_short!("created"), creator, depositor), preauth_id.clone());
        preauth_id
    }

    pub fn use_preauth(env: Env, preauth_id: BytesN<32>, amount: i128) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut preauth: Preauth = env.storage().persistent().get(&DataKey::Preauth(preauth_id.clone())).ok_or(Error::NotFound)?;

        if caller != preauth.authorized_depositor {
            return Err(Error::Unauthorized);
        }

        if preauth.used {
            return Err(Error::AlreadyUsed);
        }

        if let Some(max) = preauth.max_amount {
            if amount > max {
                return Err(Error::ExceedsMaxAmount);
            }
        }

        preauth.used = true;
        preauth.used_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&DataKey::Preauth(preauth_id.clone()), &preauth);

        env.events().publish((symbol_short!("used"), preauth_id), amount);
        Ok(())
    }

    pub fn revoke_preauth(env: Env, preauth_id: BytesN<32>) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut preauth: Preauth = env.storage().persistent().get(&DataKey::Preauth(preauth_id.clone())).ok_or(Error::NotFound)?;

        if caller != preauth.creator {
            return Err(Error::Unauthorized);
        }

        if preauth.used {
            return Err(Error::AlreadyUsed);
        }

        preauth.used = true;
        env.storage().persistent().set(&DataKey::Preauth(preauth_id.clone()), &preauth);

        env.events().publish((symbol_short!("revoked"), preauth_id), ());
        Ok(())
    }

    pub fn get_preauth(env: Env, preauth_id: BytesN<32>) -> Option<Preauth> {
        env.storage().persistent().get(&DataKey::Preauth(preauth_id))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_create_and_use_preauth() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DepositPreauthContract);
        let client = DepositPreauthContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let depositor = Address::generate(&env);
        let token = Address::generate(&env);

        let preauth_id = client.create_preauth(&depositor, &token, &Some(1000));
        client.use_preauth(&preauth_id, &500);

        let preauth = client.get_preauth(&preauth_id).unwrap();
        assert_eq!(preauth.used, true);
    }

    #[test]
    #[should_panic]
    fn test_use_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DepositPreauthContract);
        let client = DepositPreauthContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let depositor = Address::generate(&env);
        let token = Address::generate(&env);

        let preauth_id = client.create_preauth(&depositor, &token, &Some(1000));
        client.use_preauth(&preauth_id, &500);
        client.use_preauth(&preauth_id, &300);
    }
}
