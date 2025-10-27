use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault, Promise};
use serde::{Deserialize, Serialize};

const GRACE_PERIOD_NS: u64 = 86_400_000_000_000; // 24 hours in nanoseconds

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum AccountStatus {
    Active,
    PendingDeletion,
    Deleted,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Account {
    pub owner: AccountId,
    pub balance: Balance,
    pub status: AccountStatus,
    pub deletion_request_time: u64,
    pub beneficiary: Option<AccountId>,
    pub created_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AccountDeleteContract {
    accounts: UnorderedMap<AccountId, Account>,
}

#[near_bindgen]
impl AccountDeleteContract {
    #[init]
    pub fn new() -> Self {
        Self {
            accounts: UnorderedMap::new(b"a"),
        }
    }

    pub fn create_account(&mut self) {
        let owner = env::predecessor_account_id();

        assert!(
            !self.accounts.get(&owner).is_some(),
            "Account already exists"
        );

        let account = Account {
            owner: owner.clone(),
            balance: 0,
            status: AccountStatus::Active,
            deletion_request_time: 0,
            beneficiary: None,
            created_at: env::block_timestamp(),
        };

        self.accounts.insert(&owner, &account);
    }

    #[payable]
    pub fn deposit(&mut self) {
        let owner = env::predecessor_account_id();
        let amount = env::attached_deposit();

        let mut account = self.accounts.get(&owner).expect("Account not found");
        assert_eq!(account.status, AccountStatus::Active, "Account not active");
        assert!(amount > 0, "Deposit required");

        account.balance += amount;
        self.accounts.insert(&owner, &account);
    }

    pub fn request_deletion(&mut self, beneficiary: AccountId) {
        let owner = env::predecessor_account_id();

        let mut account = self.accounts.get(&owner).expect("Account not found");
        assert_eq!(account.status, AccountStatus::Active, "Account not active");
        assert_ne!(owner, beneficiary, "Cannot be own beneficiary");

        account.status = AccountStatus::PendingDeletion;
        account.deletion_request_time = env::block_timestamp();
        account.beneficiary = Some(beneficiary);

        self.accounts.insert(&owner, &account);
    }

    pub fn cancel_deletion(&mut self) {
        let owner = env::predecessor_account_id();

        let mut account = self.accounts.get(&owner).expect("Account not found");
        assert_eq!(
            account.status,
            AccountStatus::PendingDeletion,
            "No deletion pending"
        );

        account.status = AccountStatus::Active;
        account.deletion_request_time = 0;
        account.beneficiary = None;

        self.accounts.insert(&owner, &account);
    }

    pub fn execute_deletion(&mut self, account_id: AccountId) -> Promise {
        let mut account = self.accounts.get(&account_id).expect("Account not found");

        assert_eq!(
            account.status,
            AccountStatus::PendingDeletion,
            "No deletion pending"
        );

        let elapsed = env::block_timestamp() - account.deletion_request_time;
        assert!(elapsed >= GRACE_PERIOD_NS, "Grace period not elapsed");

        let beneficiary = account.beneficiary.clone().expect("No beneficiary set");
        let balance = account.balance;

        account.status = AccountStatus::Deleted;
        account.balance = 0;

        self.accounts.insert(&account_id, &account);

        if balance > 0 {
            Promise::new(beneficiary).transfer(balance)
        } else {
            Promise::new(beneficiary).transfer(0)
        }
    }

    pub fn get_account(&self, account_id: AccountId) -> Option<Account> {
        self.accounts.get(&account_id)
    }

    pub fn can_delete(&self, account_id: AccountId) -> bool {
        if let Some(account) = self.accounts.get(&account_id) {
            if account.status == AccountStatus::PendingDeletion {
                let elapsed = env::block_timestamp() - account.deletion_request_time;
                elapsed >= GRACE_PERIOD_NS
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn get_time_until_deletion(&self, account_id: AccountId) -> u64 {
        if let Some(account) = self.accounts.get(&account_id) {
            if account.status == AccountStatus::PendingDeletion {
                let elapsed = env::block_timestamp() - account.deletion_request_time;
                if elapsed >= GRACE_PERIOD_NS {
                    0
                } else {
                    GRACE_PERIOD_NS - elapsed
                }
            } else {
                0
            }
        } else {
            0
        }
    }
}
