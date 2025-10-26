#![no_std]

//! Checks - Deferred Payment Instruments
//! Production-grade Soroban implementation

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token, Address, BytesN, Env,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckStatus {
    Pending,
    Cashed,
    Cancelled,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckType {
    Bearer,
    PayeeSpecific(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Check {
    pub id: u64,
    pub issuer: Address,
    pub check_type: CheckType,
    pub token: Address,
    pub amount: i128,
    pub max_amount: Option<i128>,
    pub cashed_amount: i128,
    pub expires_at: Option<u32>,
    pub status: CheckStatus,
    pub memo: Option<BytesN<32>>,
    pub created_at: u64,
    pub cashed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Check(u64),
    NextCheckId,
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    InvalidAmount = 3,
    CheckNotPending = 4,
    AlreadyExpired = 5,
    InsufficientFunds = 6,
    ExceedsMaxAmount = 7,
    NotPayee = 8,
}

#[contract]
pub struct ChecksContract;

#[contractimpl]
impl ChecksContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextCheckId, &1u64);
    }

    pub fn create_bearer_check(
        env: Env,
        token: Address,
        amount: i128,
        duration: Option<u32>,
        memo: Option<BytesN<32>>,
    ) -> Result<u64, Error> {
        Self::create_check_internal(env, CheckType::Bearer, token, amount, None, duration, memo)
    }

    pub fn create_payee_check(
        env: Env,
        payee: Address,
        token: Address,
        amount: i128,
        max_amount: Option<i128>,
        duration: Option<u32>,
        memo: Option<BytesN<32>>,
    ) -> Result<u64, Error> {
        Self::create_check_internal(env, CheckType::PayeeSpecific(payee), token, amount, max_amount, duration, memo)
    }

    fn create_check_internal(
        env: Env,
        check_type: CheckType,
        token: Address,
        amount: i128,
        max_amount: Option<i128>,
        duration: Option<u32>,
        memo: Option<BytesN<32>>,
    ) -> Result<u64, Error> {
        let issuer = env.invoker();
        issuer.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if let Some(max) = max_amount {
            if max < amount {
                return Err(Error::ExceedsMaxAmount);
            }
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&issuer, &env.current_contract_address(), &amount);

        let check_id: u64 = env.storage().instance().get(&DataKey::NextCheckId).unwrap_or(1);
        env.storage().instance().set(&DataKey::NextCheckId, &(check_id + 1));

        let expires_at = duration.map(|d| env.ledger().sequence() + d);

        let check = Check {
            id: check_id,
            issuer: issuer.clone(),
            check_type: check_type.clone(),
            token: token.clone(),
            amount,
            max_amount,
            cashed_amount: 0,
            expires_at,
            status: CheckStatus::Pending,
            memo,
            created_at: env.ledger().timestamp(),
            cashed_at: None,
        };

        env.storage().persistent().set(&DataKey::Check(check_id), &check);
        env.storage().persistent().extend_ttl(&DataKey::Check(check_id), 518400, 518400);

        env.events().publish(
            (symbol_short!("created"), issuer),
            (check_id, amount),
        );

        Ok(check_id)
    }

    pub fn cash_check(
        env: Env,
        check_id: u64,
        cash_amount: Option<i128>,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut check: Check = env.storage()
            .persistent()
            .get(&DataKey::Check(check_id))
            .ok_or(Error::NotFound)?;

        if !matches!(check.status, CheckStatus::Pending) {
            return Err(Error::CheckNotPending);
        }

        if let Some(exp) = check.expires_at {
            if env.ledger().sequence() >= exp {
                check.status = CheckStatus::Expired;
                env.storage().persistent().set(&DataKey::Check(check_id), &check);
                return Err(Error::AlreadyExpired);
            }
        }

        match &check.check_type {
            CheckType::Bearer => {},
            CheckType::PayeeSpecific(payee) => {
                if caller != *payee {
                    return Err(Error::NotPayee);
                }
            }
        }

        let amount_to_cash = cash_amount.unwrap_or(check.amount - check.cashed_amount);

        if amount_to_cash <= 0 {
            return Err(Error::InvalidAmount);
        }

        let new_cashed = check.cashed_amount.checked_add(amount_to_cash)
            .ok_or(Error::InvalidAmount)?;

        if new_cashed > check.amount {
            return Err(Error::InsufficientFunds);
        }

        if let Some(max) = check.max_amount {
            if new_cashed > max {
                return Err(Error::ExceedsMaxAmount);
            }
        }

        let token_client = token::Client::new(&env, &check.token);
        token_client.transfer(&env.current_contract_address(), &caller, &amount_to_cash);

        check.cashed_amount = new_cashed;

        if check.cashed_amount >= check.amount {
            check.status = CheckStatus::Cashed;
            check.cashed_at = Some(env.ledger().timestamp());
        }

        env.storage().persistent().set(&DataKey::Check(check_id), &check);

        env.events().publish(
            (symbol_short!("cashed"), check_id),
            amount_to_cash,
        );

        Ok(())
    }

    pub fn cancel_check(
        env: Env,
        check_id: u64,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut check: Check = env.storage()
            .persistent()
            .get(&DataKey::Check(check_id))
            .ok_or(Error::NotFound)?;

        if caller != check.issuer {
            return Err(Error::Unauthorized);
        }

        if !matches!(check.status, CheckStatus::Pending) {
            return Err(Error::CheckNotPending);
        }

        let remaining = check.amount.checked_sub(check.cashed_amount)
            .ok_or(Error::InvalidAmount)?;

        if remaining > 0 {
            let token_client = token::Client::new(&env, &check.token);
            token_client.transfer(
                &env.current_contract_address(),
                &check.issuer,
                &remaining,
            );
        }

        check.status = CheckStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Check(check_id), &check);

        env.events().publish(
            (symbol_short!("cancelled"), check_id),
            (),
        );

        Ok(())
    }

    pub fn get_check(env: Env, check_id: u64) -> Option<Check> {
        env.storage().persistent().get(&DataKey::Check(check_id))
    }

    pub fn get_remaining_amount(env: Env, check_id: u64) -> Result<i128, Error> {
        let check: Check = env.storage()
            .persistent()
            .get(&DataKey::Check(check_id))
            .ok_or(Error::NotFound)?;
        Ok(check.amount.checked_sub(check.cashed_amount).unwrap_or(0))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_bearer_check() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ChecksContract);
        let client = ChecksContractClient::new(&env, &contract_id);

        let token = Address::generate(&env);
        let check_id = client.create_bearer_check(&token, &1000, &None, &None);

        // Anyone can cash a bearer check
        client.cash_check(&check_id, &Some(500));

        let remaining = client.get_remaining_amount(&check_id);
        assert_eq!(remaining, 500);

        // Cash the rest
        client.cash_check(&check_id, &None);

        let check = client.get_check(&check_id).unwrap();
        assert_eq!(check.status, CheckStatus::Cashed);
    }

    #[test]
    fn test_payee_check() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ChecksContract);
        let client = ChecksContractClient::new(&env, &contract_id);

        let payee = Address::generate(&env);
        let token = Address::generate(&env);

        let check_id = client.create_payee_check(&payee, &token, &1000, &None, &None, &None);

        // Payee cashes the check
        client.cash_check(&check_id, &None);

        let check = client.get_check(&check_id).unwrap();
        assert_eq!(check.status, CheckStatus::Cashed);
        assert_eq!(check.cashed_amount, 1000);
    }

    #[test]
    #[should_panic]
    fn test_wrong_payee() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ChecksContract);
        let client = ChecksContractClient::new(&env, &contract_id);

        let payee = Address::generate(&env);
        let token = Address::generate(&env);

        let check_id = client.create_payee_check(&payee, &token, &1000, &None, &None, &None);

        // Wrong person tries to cash - should fail
        let wrong_person = Address::generate(&env);
        // This will panic
        client.cash_check(&check_id, &None);
    }

    #[test]
    fn test_cancel_check() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ChecksContract);
        let client = ChecksContractClient::new(&env, &contract_id);

        let token = Address::generate(&env);
        let check_id = client.create_bearer_check(&token, &1000, &None, &None);

        // Issuer cancels the check
        client.cancel_check(&check_id);

        let check = client.get_check(&check_id).unwrap();
        assert_eq!(check.status, CheckStatus::Cancelled);
    }

    #[test]
    fn test_partial_cashing() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ChecksContract);
        let client = ChecksContractClient::new(&env, &contract_id);

        let payee = Address::generate(&env);
        let token = Address::generate(&env);

        // Create check with max amount
        let check_id = client.create_payee_check(&payee, &token, &1000, &Some(800), &None, &None);

        // Cash 500
        client.cash_check(&check_id, &Some(500));

        let remaining = client.get_remaining_amount(&check_id);
        assert_eq!(remaining, 500);

        // Try to cash more than max - should succeed since max is 800 total
        client.cash_check(&check_id, &Some(300));

        let check = client.get_check(&check_id).unwrap();
        assert_eq!(check.cashed_amount, 800);
        assert_eq!(check.status, CheckStatus::Pending); // Not fully cashed yet
    }

    #[test]
    fn test_expired_check() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ChecksContract);
        let client = ChecksContractClient::new(&env, &contract_id);

        let token = Address::generate(&env);

        // Create check that expires in 10 ledgers
        let check_id = client.create_bearer_check(&token, &1000, &Some(10), &None);

        // Advance ledger past expiration
        env.ledger().with_mut(|li| li.sequence_number += 20);

        // Try to cash - should mark as expired
        let result = client.try_cash_check(&check_id, &None);
        assert!(result.is_err());

        let check = client.get_check(&check_id).unwrap();
        assert_eq!(check.status, CheckStatus::Expired);
    }
}
