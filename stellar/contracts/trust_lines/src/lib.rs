#![no_std]

//! TrustLines - Bilateral Credit Networks with Payment Rippling
//! Production-grade Soroban implementation for Stellar
//!
//! Built on top of Stellar's native trust line primitive with extensions:
//! - Bidirectional credit limits
//! - Balance tracking (on-chain state)
//! - Payment rippling through multiple hops
//! - Quality parameters for DEX integration

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    Address, Env, Map, Vec,
};

/// Trust line data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustLine {
    /// Account 1 (lexicographically smaller)
    pub account1: Address,
    /// Account 2 (lexicographically larger)
    pub account2: Address,
    /// Asset identifier
    pub asset: Address,
    /// Credit limit from account1 to account2
    pub limit1: i128,
    /// Credit limit from account2 to account1
    pub limit2: i128,
    /// Current balance (positive = account2 owes account1)
    pub balance: i128,
    /// Allow rippling through this trust line
    pub allow_rippling: bool,
    /// Quality in (1000 = 100%)
    pub quality_in: u32,
    /// Quality out (1000 = 100%)
    pub quality_out: u32,
}

/// Storage keys
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Trust line key: (account1, account2, asset)
    TrustLine(Address, Address, Address),
    /// Admin address
    Admin,
}

/// Errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Trust line not found
    NotFound = 1,
    /// Trust line already exists
    AlreadyExists = 2,
    /// Cannot create trust line with self
    SelfTrustLine = 3,
    /// Invalid limit (must be > 0)
    InvalidLimit = 4,
    /// Insufficient credit available
    InsufficientCredit = 5,
    /// Invalid amount (must be > 0)
    InvalidAmount = 6,
    /// Unauthorized operation
    Unauthorized = 7,
    /// Rippling not enabled
    RipplingDisabled = 8,
    /// Balance must be zero to close
    NonZeroBalance = 9,
    /// Path too long
    PathTooLong = 10,
    /// Invalid quality parameter
    InvalidQuality = 11,
}

#[contract]
pub struct TrustLinesContract;

#[contractimpl]
impl TrustLinesContract {
    /// Initialize the contract
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Create a new trust line
    ///
    /// # Arguments
    /// * `counterparty` - The other party in the trust line
    /// * `asset` - The asset for this trust line
    /// * `limit` - Credit limit to extend to counterparty
    /// * `allow_rippling` - Whether to allow payments to ripple through
    pub fn create_trust_line(
        env: Env,
        counterparty: Address,
        asset: Address,
        limit: i128,
        allow_rippling: bool,
    ) -> Result<(), Error> {
        // Authenticate caller
        let caller = env.invoker();
        caller.require_auth();

        // Validate inputs
        if caller == counterparty {
            return Err(Error::SelfTrustLine);
        }
        if limit <= 0 {
            return Err(Error::InvalidLimit);
        }

        // Order accounts for consistent storage
        let (account1, account2, limit1, limit2) = if caller < counterparty {
            (caller.clone(), counterparty.clone(), limit, 0)
        } else {
            (counterparty.clone(), caller.clone(), 0, limit)
        };

        let key = DataKey::TrustLine(account1.clone(), account2.clone(), asset.clone());

        // Check if trust line already exists
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadyExists);
        }

        // Create trust line
        let trust_line = TrustLine {
            account1: account1.clone(),
            account2: account2.clone(),
            asset: asset.clone(),
            limit1,
            limit2,
            balance: 0,
            allow_rippling,
            quality_in: 1000,
            quality_out: 1000,
        };

        // Store with TTL extension
        env.storage().persistent().set(&key, &trust_line);
        env.storage().persistent().extend_ttl(&key, 518400, 518400); // 30 days

        // Emit event
        env.events().publish(
            (symbol_short!("created"), account1.clone(), account2.clone()),
            (asset, limit),
        );

        Ok(())
    }

    /// Update trust line limit
    pub fn update_limit(
        env: Env,
        counterparty: Address,
        asset: Address,
        new_limit: i128,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        if new_limit < 0 {
            return Err(Error::InvalidLimit);
        }

        let (account1, account2) = Self::order_accounts(&caller, &counterparty);
        let key = DataKey::TrustLine(account1.clone(), account2.clone(), asset.clone());

        let mut trust_line: TrustLine = env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;

        // Update appropriate limit
        if caller == account1 {
            trust_line.limit1 = new_limit;
        } else {
            trust_line.limit2 = new_limit;
        }

        env.storage().persistent().set(&key, &trust_line);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (symbol_short!("updated"), caller, counterparty),
            new_limit,
        );

        Ok(())
    }

    /// Send payment through trust line
    pub fn send_payment(
        env: Env,
        recipient: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let (account1, account2) = Self::order_accounts(&caller, &recipient);
        let key = DataKey::TrustLine(account1.clone(), account2.clone(), asset.clone());

        let mut trust_line: TrustLine = env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;

        // Calculate new balance
        let new_balance = if caller == account1 {
            // Payment from account1 to account2: balance decreases
            trust_line.balance.checked_sub(amount).ok_or(Error::InsufficientCredit)?
        } else {
            // Payment from account2 to account1: balance increases
            trust_line.balance.checked_add(amount).ok_or(Error::InsufficientCredit)?
        };

        // Check credit limits
        if caller == account1 {
            let max_negative = -trust_line.limit1;
            if new_balance < max_negative {
                return Err(Error::InsufficientCredit);
            }
        } else {
            if new_balance > trust_line.limit2 {
                return Err(Error::InsufficientCredit);
            }
        }

        trust_line.balance = new_balance;
        env.storage().persistent().set(&key, &trust_line);

        env.events().publish(
            (symbol_short!("payment"), caller, recipient),
            (amount, new_balance),
        );

        Ok(())
    }

    /// Send payment through a path (rippling)
    pub fn send_through_path(
        env: Env,
        path: Vec<Address>,
        asset: Address,
        amount: i128,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if path.len() == 0 || path.len() > 6 {
            return Err(Error::PathTooLong);
        }

        // Process payment through each hop
        let mut current = caller.clone();
        for next in path.iter() {
            let (account1, account2) = Self::order_accounts(&current, &next);
            let key = DataKey::TrustLine(account1.clone(), account2.clone(), asset.clone());

            let mut trust_line: TrustLine = env.storage()
                .persistent()
                .get(&key)
                .ok_or(Error::NotFound)?;

            // Check rippling enabled (except for first and last hop)
            if current != caller && next != path.get_unchecked(path.len() - 1) {
                if !trust_line.allow_rippling {
                    return Err(Error::RipplingDisabled);
                }
            }

            // Update balance
            let new_balance = if current == account1 {
                trust_line.balance.checked_sub(amount).ok_or(Error::InsufficientCredit)?
            } else {
                trust_line.balance.checked_add(amount).ok_or(Error::InsufficientCredit)?
            };

            // Check limits
            if current == account1 {
                if new_balance < -trust_line.limit1 {
                    return Err(Error::InsufficientCredit);
                }
            } else {
                if new_balance > trust_line.limit2 {
                    return Err(Error::InsufficientCredit);
                }
            }

            trust_line.balance = new_balance;
            env.storage().persistent().set(&key, &trust_line);

            current = next.clone();
        }

        env.events().publish(
            (symbol_short!("ripple"), caller, current),
            (amount, path.len()),
        );

        Ok(())
    }

    /// Close trust line (must have zero balance)
    pub fn close_trust_line(
        env: Env,
        counterparty: Address,
        asset: Address,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let (account1, account2) = Self::order_accounts(&caller, &counterparty);
        let key = DataKey::TrustLine(account1.clone(), account2.clone(), asset.clone());

        let trust_line: TrustLine = env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;

        if trust_line.balance != 0 {
            return Err(Error::NonZeroBalance);
        }

        env.storage().persistent().remove(&key);

        env.events().publish(
            (symbol_short!("closed"), account1, account2),
            asset,
        );

        Ok(())
    }

    /// Update rippling settings
    pub fn set_rippling(
        env: Env,
        counterparty: Address,
        asset: Address,
        allow: bool,
    ) -> Result<(), Error> {
        let caller = env.invoker();
        caller.require_auth();

        let (account1, account2) = Self::order_accounts(&caller, &counterparty);
        let key = DataKey::TrustLine(account1.clone(), account2.clone(), asset);

        let mut trust_line: TrustLine = env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;

        // Only account1 can set rippling (by convention)
        if caller != account1 {
            return Err(Error::Unauthorized);
        }

        trust_line.allow_rippling = allow;
        env.storage().persistent().set(&key, &trust_line);

        Ok(())
    }

    /// Get trust line details
    pub fn get_trust_line(
        env: Env,
        account1: Address,
        account2: Address,
        asset: Address,
    ) -> Option<TrustLine> {
        let (acc1, acc2) = Self::order_accounts(&account1, &account2);
        let key = DataKey::TrustLine(acc1, acc2, asset);
        env.storage().persistent().get(&key)
    }

    /// Get available credit
    pub fn get_available_credit(
        env: Env,
        from: Address,
        to: Address,
        asset: Address,
    ) -> i128 {
        let (account1, account2) = Self::order_accounts(&from, &to);
        let key = DataKey::TrustLine(account1.clone(), account2.clone(), asset);

        if let Some(trust_line) = env.storage().persistent().get::<_, TrustLine>(&key) {
            if from == account1 {
                let used = if trust_line.balance < 0 { -trust_line.balance } else { 0 };
                trust_line.limit1 - used
            } else {
                let used = if trust_line.balance > 0 { trust_line.balance } else { 0 };
                trust_line.limit2 - used
            }
        } else {
            0
        }
    }

    // Helper: Order addresses consistently
    fn order_accounts(a: &Address, b: &Address) -> (Address, Address) {
        if a < b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_create_trust_line() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TrustLinesContract);
        let client = TrustLinesContractClient::new(&env, &contract_id);

        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let asset = Address::generate(&env);

        env.mock_all_auths();

        client.create_trust_line(&bob, &asset, &1000, &true);

        let trust_line = client.get_trust_line(&alice, &bob, &asset).unwrap();
        assert_eq!(trust_line.limit1 + trust_line.limit2, 1000);
        assert_eq!(trust_line.balance, 0);
    }

    #[test]
    fn test_send_payment() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TrustLinesContract);
        let client = TrustLinesContractClient::new(&env, &contract_id);

        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let asset = Address::generate(&env);

        env.mock_all_auths();

        client.create_trust_line(&bob, &asset, &1000, &true);
        client.send_payment(&bob, &asset, &100);

        let available = client.get_available_credit(&alice, &bob, &asset);
        assert_eq!(available, 900);
    }

    #[test]
    #[should_panic(expected = "InsufficientCredit")]
    fn test_insufficient_credit() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TrustLinesContract);
        let client = TrustLinesContractClient::new(&env, &contract_id);

        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let asset = Address::generate(&env);

        env.mock_all_auths();

        client.create_trust_line(&bob, &asset, &100, &true);
        client.send_payment(&bob, &asset, &200); // Should panic
    }
}
