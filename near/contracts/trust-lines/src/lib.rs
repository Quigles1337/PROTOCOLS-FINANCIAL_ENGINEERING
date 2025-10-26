use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TrustLine {
    pub account1: AccountId,
    pub account2: AccountId,
    pub limit1: Balance,
    pub limit2: Balance,
    pub balance: Balance,
    pub is_negative: bool,
    pub active: bool,
    pub created_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TrustLines {
    trust_lines: UnorderedMap<String, TrustLine>,
}

#[near_bindgen]
impl TrustLines {
    #[init]
    pub fn new() -> Self {
        Self {
            trust_lines: UnorderedMap::new(b"t"),
        }
    }

    #[payable]
    pub fn create_trust_line(&mut self, counterparty: AccountId, limit1: Balance, limit2: Balance) {
        let caller = env::predecessor_account_id();
        assert_ne!(caller, counterparty, "Cannot create trust line with self");

        let key = Self::generate_key(&caller, &counterparty);
        assert!(!self.trust_lines.get(&key).is_some(), "Trust line already exists");

        let trust_line = TrustLine {
            account1: caller,
            account2: counterparty,
            limit1,
            limit2,
            balance: 0,
            is_negative: false,
            active: true,
            created_at: env::block_timestamp(),
        };

        self.trust_lines.insert(&key, &trust_line);
    }

    pub fn update_limit(&mut self, counterparty: AccountId, new_limit: Balance) {
        let caller = env::predecessor_account_id();
        let key = Self::generate_key(&caller, &counterparty);

        let mut trust_line = self.trust_lines.get(&key).expect("Trust line not found");
        assert!(trust_line.active, "Trust line not active");

        if trust_line.account1 == caller {
            trust_line.limit1 = new_limit;
        } else {
            trust_line.limit2 = new_limit;
        }

        self.trust_lines.insert(&key, &trust_line);
    }

    pub fn ripple_payment(&mut self, receiver: AccountId, amount: Balance) {
        let sender = env::predecessor_account_id();
        let key = Self::generate_key(&sender, &receiver);

        let mut trust_line = self.trust_lines.get(&key).expect("Trust line not found");
        assert!(trust_line.active, "Trust line not active");
        assert!(amount > 0, "Amount must be positive");

        let is_account1 = trust_line.account1 == sender;

        if is_account1 {
            if trust_line.is_negative {
                if amount >= trust_line.balance {
                    trust_line.balance = amount - trust_line.balance;
                    trust_line.is_negative = false;
                } else {
                    trust_line.balance -= amount;
                }
            } else {
                trust_line.balance += amount;
                assert!(trust_line.balance <= trust_line.limit1, "Balance exceeds limit");
            }
        } else {
            if !trust_line.is_negative {
                if amount >= trust_line.balance {
                    trust_line.balance = amount - trust_line.balance;
                    trust_line.is_negative = true;
                    assert!(trust_line.balance <= trust_line.limit2, "Balance exceeds limit");
                } else {
                    trust_line.balance -= amount;
                }
            } else {
                trust_line.balance += amount;
                assert!(trust_line.balance <= trust_line.limit2, "Balance exceeds limit");
            }
        }

        self.trust_lines.insert(&key, &trust_line);
    }

    pub fn close_trust_line(&mut self, counterparty: AccountId) {
        let caller = env::predecessor_account_id();
        let key = Self::generate_key(&caller, &counterparty);

        let mut trust_line = self.trust_lines.get(&key).expect("Trust line not found");
        assert_eq!(trust_line.balance, 0, "Balance must be zero");

        trust_line.active = false;
        self.trust_lines.insert(&key, &trust_line);
    }

    pub fn get_trust_line(&self, account1: AccountId, account2: AccountId) -> Option<TrustLine> {
        let key = Self::generate_key(&account1, &account2);
        self.trust_lines.get(&key)
    }

    fn generate_key(addr1: &AccountId, addr2: &AccountId) -> String {
        if addr1 < addr2 {
            format!("{}:{}", addr1, addr2)
        } else {
            format!("{}:{}", addr2, addr1)
        }
    }
}
