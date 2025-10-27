use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault, Promise};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Escrow {
    pub sender: AccountId,
    pub receiver: AccountId,
    pub amount: Balance,
    pub release_time: u64,
    pub cancel_time: u64,
    pub condition_hash: Option<Vec<u8>>,
    pub status: EscrowStatus,
    pub created_at: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum EscrowStatus {
    Active,
    Executed,
    Cancelled,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct EscrowContract {
    escrows: UnorderedMap<u64, Escrow>,
    next_id: u64,
}

#[near_bindgen]
impl EscrowContract {
    #[init]
    pub fn new() -> Self {
        Self {
            escrows: UnorderedMap::new(b"e"),
            next_id: 0,
        }
    }

    #[payable]
    pub fn create_time_locked(&mut self, receiver: AccountId, release_time: u64, cancel_time: u64) -> u64 {
        self.create_escrow_internal(receiver, release_time, cancel_time, None)
    }

    #[payable]
    pub fn create_hash_locked(
        &mut self,
        receiver: AccountId,
        release_time: u64,
        cancel_time: u64,
        condition_hash: Vec<u8>,
    ) -> u64 {
        assert_eq!(condition_hash.len(), 32, "Hash must be 32 bytes");
        self.create_escrow_internal(receiver, release_time, cancel_time, Some(condition_hash))
    }

    fn create_escrow_internal(
        &mut self,
        receiver: AccountId,
        release_time: u64,
        cancel_time: u64,
        condition_hash: Option<Vec<u8>>,
    ) -> u64 {
        let sender = env::predecessor_account_id();
        let amount = env::attached_deposit();

        assert!(amount > 0, "Deposit required");
        assert!(release_time >= env::block_timestamp(), "Invalid release time");
        assert!(cancel_time > release_time, "Cancel time must be after release time");

        let escrow_id = self.next_id;
        self.next_id += 1;

        let escrow = Escrow {
            sender,
            receiver,
            amount,
            release_time,
            cancel_time,
            condition_hash,
            status: EscrowStatus::Active,
            created_at: env::block_timestamp(),
        };

        self.escrows.insert(&escrow_id, &escrow);
        escrow_id
    }

    pub fn execute_escrow(&mut self, escrow_id: u64, preimage: Option<Vec<u8>>) -> Promise {
        let receiver = env::predecessor_account_id();

        let mut escrow = self.escrows.get(&escrow_id).expect("Escrow not found");
        assert_eq!(escrow.receiver, receiver, "Not authorized");
        assert_eq!(escrow.status, EscrowStatus::Active, "Escrow not active");
        assert!(env::block_timestamp() >= escrow.release_time, "Not released yet");
        assert!(env::block_timestamp() < escrow.cancel_time, "Escrow expired");

        // Verify hash condition if present
        if let Some(hash) = &escrow.condition_hash {
            let provided_preimage = preimage.expect("Preimage required");
            let computed_hash = env::sha256(&provided_preimage);
            assert_eq!(&computed_hash[..], &hash[..], "Invalid preimage");
        }

        escrow.status = EscrowStatus::Executed;
        self.escrows.insert(&escrow_id, &escrow);

        Promise::new(receiver).transfer(escrow.amount)
    }

    pub fn cancel_escrow(&mut self, escrow_id: u64) -> Promise {
        let sender = env::predecessor_account_id();

        let mut escrow = self.escrows.get(&escrow_id).expect("Escrow not found");
        assert_eq!(escrow.sender, sender, "Not authorized");
        assert_eq!(escrow.status, EscrowStatus::Active, "Escrow not active");
        assert!(env::block_timestamp() >= escrow.cancel_time, "Cannot cancel yet");

        escrow.status = EscrowStatus::Cancelled;
        self.escrows.insert(&escrow_id, &escrow);

        Promise::new(sender).transfer(escrow.amount)
    }

    pub fn get_escrow(&self, escrow_id: u64) -> Option<Escrow> {
        self.escrows.get(&escrow_id)
    }

    pub fn is_executable(&self, escrow_id: u64) -> bool {
        if let Some(escrow) = self.escrows.get(&escrow_id) {
            let now = env::block_timestamp();
            escrow.status == EscrowStatus::Active
                && now >= escrow.release_time
                && now < escrow.cancel_time
        } else {
            false
        }
    }
}
