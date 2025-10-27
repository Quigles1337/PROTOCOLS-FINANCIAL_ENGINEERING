use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum PreauthStatus {
    Active,
    Used,
    Revoked,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Preauth {
    pub authorizer: AccountId,
    pub authorized: AccountId,
    pub asset: String,
    pub max_amount: Balance,
    pub expiration: u64,
    pub status: PreauthStatus,
    pub created_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DepositPreauthContract {
    preauths: UnorderedMap<u64, Preauth>,
    next_id: u64,
}

#[near_bindgen]
impl DepositPreauthContract {
    #[init]
    pub fn new() -> Self {
        Self {
            preauths: UnorderedMap::new(b"p"),
            next_id: 0,
        }
    }

    pub fn create_preauth(
        &mut self,
        authorized: AccountId,
        asset: String,
        max_amount: Balance,
        expiration: u64,
    ) -> u64 {
        let authorizer = env::predecessor_account_id();

        assert_ne!(authorizer, authorized, "Cannot preauth self");
        assert!(max_amount > 0, "Invalid max amount");
        assert!(expiration > env::block_timestamp(), "Invalid expiration");

        let preauth_id = self.next_id;
        self.next_id += 1;

        let preauth = Preauth {
            authorizer,
            authorized,
            asset,
            max_amount,
            expiration,
            status: PreauthStatus::Active,
            created_at: env::block_timestamp(),
        };

        self.preauths.insert(&preauth_id, &preauth);
        preauth_id
    }

    pub fn use_preauth(&mut self, preauth_id: u64, amount: Balance) {
        let authorized = env::predecessor_account_id();

        let mut preauth = self.preauths.get(&preauth_id).expect("Preauth not found");

        assert_eq!(preauth.authorized, authorized, "Not authorized");
        assert_eq!(preauth.status, PreauthStatus::Active, "Preauth not active");
        assert!(
            env::block_timestamp() < preauth.expiration,
            "Preauth expired"
        );
        assert!(amount <= preauth.max_amount, "Amount exceeds limit");

        preauth.status = PreauthStatus::Used;
        self.preauths.insert(&preauth_id, &preauth);
    }

    pub fn revoke_preauth(&mut self, preauth_id: u64) {
        let authorizer = env::predecessor_account_id();

        let mut preauth = self.preauths.get(&preauth_id).expect("Preauth not found");

        assert_eq!(preauth.authorizer, authorizer, "Not authorized");
        assert_eq!(preauth.status, PreauthStatus::Active, "Already used/revoked");

        preauth.status = PreauthStatus::Revoked;
        self.preauths.insert(&preauth_id, &preauth);
    }

    pub fn get_preauth(&self, preauth_id: u64) -> Option<Preauth> {
        self.preauths.get(&preauth_id)
    }

    pub fn is_valid(&self, preauth_id: u64) -> bool {
        if let Some(preauth) = self.preauths.get(&preauth_id) {
            let now = env::block_timestamp();
            preauth.status == PreauthStatus::Active && now < preauth.expiration
        } else {
            false
        }
    }
}
