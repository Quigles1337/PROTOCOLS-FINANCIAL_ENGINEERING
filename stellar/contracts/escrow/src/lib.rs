#![no_std]

//! Escrow - Conditional Hash-Locked Time-Locked Contracts (HTLCs)
//! Production-grade Soroban implementation
//!
//! Features:
//! - Time-locked escrow (release after specific ledger)
//! - Hash-locked escrow (HTLC with preimage verification)
//! - Combined time+hash locks for atomic swaps
//! - Expiration with sender cancellation
//! - Clawback mechanism for compliance

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token, Address, BytesN, Env, Vec, vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Pending,
    Completed,
    Cancelled,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Condition {
    None,
    HashLock(BytesN<32>),
    TimeLock(u32),
    Combined(BytesN<32>, u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub id: u64,
    pub sender: Address,
    pub recipient: Address,
    pub token: Address,
    pub amount: i128,
    pub condition: Condition,
    pub expires_at: u32,
    pub status: EscrowStatus,
    pub memo: Option<BytesN<32>>,
    pub allow_clawback: bool,
    pub created_at: u64,
    pub finished_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Escrow(u64),
    NextEscrowId,
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    InvalidAmount = 3,
    EscrowNotPending = 4,
    NotExpired = 5,
    AlreadyExpired = 6,
    InvalidPreimage = 7,
    TimeNotReached = 8,
    ClawbackNotAllowed = 9,
    InvalidCondition = 10,
    HashMismatch = 11,
}

#[contract]
pub struct EscrowContract;


// File created successfully - 83 lines written

#[contractimpl]
impl EscrowContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextEscrowId, &1u64);
    }

    pub fn create_simple(
        env: Env,
        recipient: Address,
        token: Address,
        amount: i128,
        duration: u32,
    ) -> Result<u64, Error> {
        Self::create_escrow_internal(
            env, recipient, token, amount,
            Condition::None, duration, None, false,
        )
    }

    pub fn create_hash_locked(
        env: Env,
        recipient: Address,
        token: Address,
        amount: i128,
        hash_lock: BytesN<32>,
        duration: u32,
    ) -> Result<u64, Error> {
        Self::create_escrow_internal(
            env, recipient, token, amount,
            Condition::HashLock(hash_lock), duration, None, false,
        )
    }

    pub fn create_time_locked(
        env: Env,
        recipient: Address,
        token: Address,
        amount: i128,
        unlock_at: u32,
        expiration: u32,
    ) -> Result<u64, Error> {
        if unlock_at >= expiration {
            return Err(Error::InvalidCondition);
        }
        let duration = expiration.checked_sub(env.ledger().sequence())
            .ok_or(Error::InvalidAmount)?;
        Self::create_escrow_internal(
            env, recipient, token, amount,
            Condition::TimeLock(unlock_at), duration, None, false,
        )
    }

    pub fn create_atomic_swap(
        env: Env,
        recipient: Address,
        token: Address,
        amount: i128,
        hash_lock: BytesN<32>,
        unlock_at: u32,
        expiration: u32,
    ) -> Result<u64, Error> {
        if unlock_at >= expiration {
            return Err(Error::InvalidCondition);
        }
        let duration = expiration.checked_sub(env.ledger().sequence())
            .ok_or(Error::InvalidAmount)?;
        Self::create_escrow_internal(
            env, recipient, token, amount,
            Condition::Combined(hash_lock, unlock_at), duration, None, false,
        )
    }

    pub fn create_escrow(
        env: Env,
        recipient: Address,
        token: Address,
        amount: i128,
        condition: Condition,
        duration: u32,
        memo: Option<BytesN<32>>,
        allow_clawback: bool,
    ) -> Result<u64, Error> {
        Self::create_escrow_internal(
            env, recipient, token, amount, condition,
            duration, memo, allow_clawback,
        )
    }

    fn create_escrow_internal(
        env: Env,
        recipient: Address,
        token: Address,
        amount: i128,
        condition: Condition,
        duration: u32,
        memo: Option<BytesN<32>>,
        allow_clawback: bool,
    ) -> Result<u64, Error> {
        let sender = env.invoker();
        sender.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&sender, &env.current_contract_address(), &amount);

        let escrow_id: u64 = env.storage()
            .instance()
            .get(&DataKey::NextEscrowId)
            .unwrap_or(1);
        env.storage().instance().set(&DataKey::NextEscrowId, &(escrow_id + 1));

        let escrow = Escrow {
            id: escrow_id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            amount,
            condition,
            expires_at: env.ledger().sequence() + duration,
            status: EscrowStatus::Pending,
            memo,
            allow_clawback,
            created_at: env.ledger().timestamp(),
            finished_at: None,
        };

        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage().persistent().extend_ttl(&DataKey::Escrow(escrow_id), 518400, 518400);

        env.events().publish(
            (symbol_short!("created"), sender, recipient),
            (escrow_id, amount),
        );

        Ok(escrow_id)
    }

    pub fn execute(
        env: Env,
        escrow_id: u64,
        preimage: Option<BytesN<32>>,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let mut escrow: Escrow = env.storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::NotFound)?;

        if caller != escrow.recipient {
            return Err(Error::Unauthorized);
        }

        if !matches!(escrow.status, EscrowStatus::Pending) {
            return Err(Error::EscrowNotPending);
        }

        if env.ledger().sequence() >= escrow.expires_at {
            return Err(Error::AlreadyExpired);
        }

        match &escrow.condition {
            Condition::None => {},
            Condition::HashLock(hash) => {
                let provided_preimage = preimage.ok_or(Error::InvalidPreimage)?;
                let computed_hash = env.crypto().sha256(&provided_preimage);
                if computed_hash != *hash {
                    return Err(Error::HashMismatch);
                }
            },
            Condition::TimeLock(unlock_at) => {
                if env.ledger().sequence() < *unlock_at {
                    return Err(Error::TimeNotReached);
                }
            },
            Condition::Combined(hash, unlock_at) => {
                if env.ledger().sequence() < *unlock_at {
                    return Err(Error::TimeNotReached);
                }
                let provided_preimage = preimage.ok_or(Error::InvalidPreimage)?;
                let computed_hash = env.crypto().sha256(&provided_preimage);
                if computed_hash != *hash {
                    return Err(Error::HashMismatch);
                }
            },
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.recipient,
            &escrow.amount,
        );

        escrow.status = EscrowStatus::Completed;
        escrow.finished_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);

        env.events().publish(
            (symbol_short!("executed"), escrow_id),
            escrow.amount,
        );

        Ok(())
    }

    pub fn cancel_expired(env: Env, escrow_id: u64) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).ok_or(Error::NotFound)?;
        if caller != escrow.sender { return Err(Error::Unauthorized); }
        if !matches!(escrow.status, EscrowStatus::Pending) { return Err(Error::EscrowNotPending); }
        if env.ledger().sequence() < escrow.expires_at { return Err(Error::NotExpired); }
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&env.current_contract_address(), &escrow.sender, &escrow.amount);
        escrow.status = EscrowStatus::Expired;
        escrow.finished_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.events().publish((symbol_short!("expired"), escrow_id), ());
        Ok(())
    }

    pub fn clawback(env: Env, escrow_id: u64) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).ok_or(Error::NotFound)?;
        if caller != escrow.sender { return Err(Error::Unauthorized); }
        if !escrow.allow_clawback { return Err(Error::ClawbackNotAllowed); }
        if !matches!(escrow.status, EscrowStatus::Pending) { return Err(Error::EscrowNotPending); }
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&env.current_contract_address(), &escrow.sender, &escrow.amount);
        escrow.status = EscrowStatus::Cancelled;
        escrow.finished_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.events().publish((symbol_short!("clawback"), escrow_id), ());
        Ok(())
    }

    pub fn extend_expiration(env: Env, escrow_id: u64, additional_duration: u32) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).ok_or(Error::NotFound)?;
        if caller != escrow.sender { return Err(Error::Unauthorized); }
        if !matches!(escrow.status, EscrowStatus::Pending) { return Err(Error::EscrowNotPending); }
        escrow.expires_at = escrow.expires_at.checked_add(additional_duration).ok_or(Error::InvalidAmount)?;
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
        env.events().publish((symbol_short!("extended"), escrow_id), escrow.expires_at);
        Ok(())
    }

    pub fn get_escrow(env: Env, escrow_id: u64) -> Option<Escrow> {
        env.storage().persistent().get(&DataKey::Escrow(escrow_id))
    }

    pub fn can_execute(env: Env, escrow_id: u64, preimage: Option<BytesN<32>>) -> Result<bool, Error> {
        let escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(escrow_id)).ok_or(Error::NotFound)?;
        if !matches!(escrow.status, EscrowStatus::Pending) { return Ok(false); }
        if env.ledger().sequence() >= escrow.expires_at { return Ok(false); }
        match &escrow.condition {
            Condition::None => Ok(true),
            Condition::HashLock(hash) => {
                if let Some(provided_preimage) = preimage {
                    Ok(env.crypto().sha256(&provided_preimage) == *hash)
                } else { Ok(false) }
            },
            Condition::TimeLock(unlock_at) => Ok(env.ledger().sequence() >= *unlock_at),
            Condition::Combined(hash, unlock_at) => {
                if env.ledger().sequence() < *unlock_at { return Ok(false); }
                if let Some(provided_preimage) = preimage {
                    Ok(env.crypto().sha256(&provided_preimage) == *hash)
                } else { Ok(false) }
            },
        }
    }

    pub fn create_batch(env: Env, recipients: Vec<Address>, tokens: Vec<Address>, amounts: Vec<i128>, hash_lock: BytesN<32>, duration: u32) -> Result<Vec<u64>, Error> {
        if recipients.len() != tokens.len() || tokens.len() != amounts.len() { return Err(Error::InvalidAmount); }
        let mut escrow_ids = vec![&env];
        for i in 0..recipients.len() {
            let escrow_id = Self::create_hash_locked(env.clone(), recipients.get(i).ok_or(Error::InvalidAmount)?, tokens.get(i).ok_or(Error::InvalidAmount)?, amounts.get(i).ok_or(Error::InvalidAmount)?, hash_lock.clone(), duration)?;
            escrow_ids.push_back(escrow_id);
        }
        Ok(escrow_ids)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, Address, BytesN, Env};

    #[test]
    fn test_simple_escrow() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let escrow_id = client.create_simple(&recipient, &token, &1000, &100);
        let escrow = client.get_escrow(&escrow_id).unwrap();
        assert_eq\!(escrow.amount, 1000);
        assert_eq\!(escrow.status, EscrowStatus::Pending);
    }

    #[test]
    fn test_hash_locked_escrow() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let preimage = BytesN::from_array(&env, &[1u8; 32]);
        let hash = env.crypto().sha256(&preimage);
        let escrow_id = client.create_hash_locked(&recipient, &token, &1000, &hash, &100);
        client.execute(&escrow_id, &Some(preimage));
        let escrow = client.get_escrow(&escrow_id).unwrap();
        assert_eq\!(escrow.status, EscrowStatus::Completed);
    }

    #[test]
    #[should_panic(expected = "HashMismatch")]
    fn test_wrong_preimage() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let hash = BytesN::from_array(&env, &[2u8; 32]);
        let escrow_id = client.create_hash_locked(&recipient, &token, &1000, &hash, &100);
        let wrong_preimage = BytesN::from_array(&env, &[1u8; 32]);
        client.execute(&escrow_id, &Some(wrong_preimage));
    }

    #[test]
    fn test_time_locked_escrow() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let current_ledger = env.ledger().sequence();
        let unlock_at = current_ledger + 50;
        let expiration = current_ledger + 100;
        let escrow_id = client.create_time_locked(&recipient, &token, &1000, &unlock_at, &expiration);
        let can_execute_early = client.can_execute(&escrow_id, &None);
        assert_eq\!(can_execute_early, false);
        env.ledger().with_mut(|li| li.sequence_number = unlock_at);
        let can_execute_now = client.can_execute(&escrow_id, &None);
        assert_eq\!(can_execute_now, true);
        client.execute(&escrow_id, &None);
        let escrow = client.get_escrow(&escrow_id).unwrap();
        assert_eq\!(escrow.status, EscrowStatus::Completed);
    }

    #[test]
    fn test_cancel_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let escrow_id = client.create_simple(&recipient, &token, &1000, &10);
        env.ledger().with_mut(|li| li.sequence_number += 20);
        client.cancel_expired(&escrow_id);
        let escrow = client.get_escrow(&escrow_id).unwrap();
        assert_eq\!(escrow.status, EscrowStatus::Expired);
    }

    #[test]
    fn test_clawback() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let escrow_id = client.create_escrow(&recipient, &token, &1000, &Condition::None, &100, &None, &true);
        client.clawback(&escrow_id);
        let escrow = client.get_escrow(&escrow_id).unwrap();
        assert_eq\!(escrow.status, EscrowStatus::Cancelled);
    }

    #[test]
    fn test_atomic_swap() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);
        let recipient = Address::generate(&env);
        let token = Address::generate(&env);
        let preimage = BytesN::from_array(&env, &[42u8; 32]);
        let hash = env.crypto().sha256(&preimage);
        let current_ledger = env.ledger().sequence();
        let unlock_at = current_ledger + 10;
        let expiration = current_ledger + 100;
        let escrow_id = client.create_atomic_swap(&recipient, &token, &1000, &hash, &unlock_at, &expiration);
        env.ledger().with_mut(|li| li.sequence_number = unlock_at);
        client.execute(&escrow_id, &Some(preimage));
        let escrow = client.get_escrow(&escrow_id).unwrap();
        assert_eq\!(escrow.status, EscrowStatus::Completed);
    }
}
