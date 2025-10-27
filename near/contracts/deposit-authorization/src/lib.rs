use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum KYCTier {
    Basic,
    Standard,
    Premium,
    Institutional,
}

impl KYCTier {
    pub fn max_amount(&self) -> Balance {
        match self {
            KYCTier::Basic => 1_000_000_000_000_000_000_000_000,        // 1 NEAR
            KYCTier::Standard => 10_000_000_000_000_000_000_000_000,     // 10 NEAR
            KYCTier::Premium => 100_000_000_000_000_000_000_000_000,     // 100 NEAR
            KYCTier::Institutional => 1_000_000_000_000_000_000_000_000_000, // 1000 NEAR
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Authorization {
    pub authorizer: AccountId,
    pub authorized: AccountId,
    pub asset: String,
    pub max_amount: Balance,
    pub used_amount: Balance,
    pub expiration: u64,
    pub tier: KYCTier,
    pub active: bool,
    pub created_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DepositAuthContract {
    authorizations: UnorderedMap<String, Authorization>,
}

#[near_bindgen]
impl DepositAuthContract {
    #[init]
    pub fn new() -> Self {
        Self {
            authorizations: UnorderedMap::new(b"a"),
        }
    }

    pub fn create_authorization(
        &mut self,
        authorized: AccountId,
        asset: String,
        max_amount: Balance,
        expiration: u64,
        tier: KYCTier,
    ) {
        let authorizer = env::predecessor_account_id();

        assert_ne!(authorizer, authorized, "Cannot authorize self");
        assert!(max_amount > 0, "Invalid max amount");
        assert!(expiration > env::block_timestamp(), "Invalid expiration");
        assert!(
            max_amount <= tier.max_amount(),
            "Amount exceeds tier limit"
        );

        let key = Self::generate_key(&authorizer, &authorized, &asset);
        assert!(
            !self.authorizations.get(&key).is_some(),
            "Authorization already exists"
        );

        let authorization = Authorization {
            authorizer,
            authorized,
            asset,
            max_amount,
            used_amount: 0,
            expiration,
            tier,
            active: true,
            created_at: env::block_timestamp(),
        };

        self.authorizations.insert(&key, &authorization);
    }

    pub fn validate_deposit(
        &self,
        authorizer: AccountId,
        authorized: AccountId,
        asset: String,
        amount: Balance,
    ) -> bool {
        let key = Self::generate_key(&authorizer, &authorized, &asset);

        if let Some(auth) = self.authorizations.get(&key) {
            let now = env::block_timestamp();
            auth.active
                && now < auth.expiration
                && (auth.used_amount + amount) <= auth.max_amount
        } else {
            false
        }
    }

    pub fn use_authorization(
        &mut self,
        authorizer: AccountId,
        asset: String,
        amount: Balance,
    ) {
        let authorized = env::predecessor_account_id();
        let key = Self::generate_key(&authorizer, &authorized, &asset);

        let mut auth = self
            .authorizations
            .get(&key)
            .expect("Authorization not found");

        assert!(auth.active, "Authorization not active");
        assert!(
            env::block_timestamp() < auth.expiration,
            "Authorization expired"
        );
        assert!(
            auth.used_amount + amount <= auth.max_amount,
            "Amount exceeds limit"
        );

        auth.used_amount += amount;
        self.authorizations.insert(&key, &auth);
    }

    pub fn revoke_authorization(&mut self, authorized: AccountId, asset: String) {
        let authorizer = env::predecessor_account_id();
        let key = Self::generate_key(&authorizer, &authorized, &asset);

        let mut auth = self
            .authorizations
            .get(&key)
            .expect("Authorization not found");

        assert_eq!(auth.authorizer, authorizer, "Not authorized");
        assert!(auth.active, "Already revoked");

        auth.active = false;
        self.authorizations.insert(&key, &auth);
    }

    pub fn update_tier(&mut self, authorized: AccountId, asset: String, new_tier: KYCTier) {
        let authorizer = env::predecessor_account_id();
        let key = Self::generate_key(&authorizer, &authorized, &asset);

        let mut auth = self
            .authorizations
            .get(&key)
            .expect("Authorization not found");

        assert_eq!(auth.authorizer, authorizer, "Not authorized");
        assert!(auth.active, "Authorization not active");
        assert!(
            auth.max_amount <= new_tier.max_amount(),
            "Amount exceeds new tier limit"
        );

        auth.tier = new_tier;
        self.authorizations.insert(&key, &auth);
    }

    pub fn get_authorization(
        &self,
        authorizer: AccountId,
        authorized: AccountId,
        asset: String,
    ) -> Option<Authorization> {
        let key = Self::generate_key(&authorizer, &authorized, &asset);
        self.authorizations.get(&key)
    }

    pub fn get_remaining_amount(
        &self,
        authorizer: AccountId,
        authorized: AccountId,
        asset: String,
    ) -> Balance {
        let key = Self::generate_key(&authorizer, &authorized, &asset);
        if let Some(auth) = self.authorizations.get(&key) {
            auth.max_amount - auth.used_amount
        } else {
            0
        }
    }

    fn generate_key(authorizer: &AccountId, authorized: &AccountId, asset: &String) -> String {
        format!("{}:{}:{}", authorizer, authorized, asset)
    }
}
