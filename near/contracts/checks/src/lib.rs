use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault, Promise};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum CheckStatus {
    Active,
    Cashed,
    Cancelled,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Check {
    pub sender: AccountId,
    pub receiver: AccountId,
    pub amount: Balance,
    pub expiration: u64,
    pub status: CheckStatus,
    pub cashed_amount: Balance,
    pub created_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ChecksContract {
    checks: UnorderedMap<u64, Check>,
    next_id: u64,
}

#[near_bindgen]
impl ChecksContract {
    #[init]
    pub fn new() -> Self {
        Self {
            checks: UnorderedMap::new(b"c"),
            next_id: 0,
        }
    }

    #[payable]
    pub fn create_check(&mut self, receiver: AccountId, expiration: u64) -> u64 {
        let sender = env::predecessor_account_id();
        let amount = env::attached_deposit();

        assert!(amount > 0, "Deposit required");
        assert_ne!(sender, receiver, "Cannot create check to self");
        assert!(expiration > env::block_timestamp(), "Invalid expiration");

        let check_id = self.next_id;
        self.next_id += 1;

        let check = Check {
            sender,
            receiver,
            amount,
            expiration,
            status: CheckStatus::Active,
            cashed_amount: 0,
            created_at: env::block_timestamp(),
        };

        self.checks.insert(&check_id, &check);
        check_id
    }

    pub fn cash_check(&mut self, check_id: u64, amount: Balance) -> Promise {
        let receiver = env::predecessor_account_id();

        let mut check = self.checks.get(&check_id).expect("Check not found");
        assert_eq!(check.receiver, receiver, "Not authorized");
        assert_eq!(check.status, CheckStatus::Active, "Check not active");
        assert!(env::block_timestamp() < check.expiration, "Check expired");

        let remaining = check.amount - check.cashed_amount;
        assert!(amount > 0 && amount <= remaining, "Invalid amount");

        check.cashed_amount += amount;

        if check.cashed_amount == check.amount {
            check.status = CheckStatus::Cashed;
        }

        self.checks.insert(&check_id, &check);

        Promise::new(receiver).transfer(amount)
    }

    pub fn cancel_check(&mut self, check_id: u64) -> Promise {
        let sender = env::predecessor_account_id();

        let mut check = self.checks.get(&check_id).expect("Check not found");
        assert_eq!(check.sender, sender, "Not authorized");
        assert_eq!(check.status, CheckStatus::Active, "Check not active");

        let remaining = check.amount - check.cashed_amount;

        check.status = CheckStatus::Cancelled;
        self.checks.insert(&check_id, &check);

        Promise::new(sender).transfer(remaining)
    }

    pub fn expire_check(&mut self, check_id: u64) -> Promise {
        let mut check = self.checks.get(&check_id).expect("Check not found");
        assert_eq!(check.status, CheckStatus::Active, "Check not active");
        assert!(env::block_timestamp() >= check.expiration, "Not expired yet");

        let remaining = check.amount - check.cashed_amount;

        check.status = CheckStatus::Cancelled;
        self.checks.insert(&check_id, &check);

        Promise::new(check.sender.clone()).transfer(remaining)
    }

    pub fn get_check(&self, check_id: u64) -> Option<Check> {
        self.checks.get(&check_id)
    }

    pub fn get_remaining_amount(&self, check_id: u64) -> Balance {
        if let Some(check) = self.checks.get(&check_id) {
            check.amount - check.cashed_amount
        } else {
            0
        }
    }
}
