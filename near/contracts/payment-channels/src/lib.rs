use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault, Promise};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Channel {
    pub sender: AccountId,
    pub receiver: AccountId,
    pub balance: Balance,
    pub total_claimed: Balance,
    pub expiration: u64,
    pub status: ChannelStatus,
    pub created_at: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum ChannelStatus {
    Open,
    Closed,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PaymentChannels {
    channels: UnorderedMap<u64, Channel>,
    next_id: u64,
}

#[near_bindgen]
impl PaymentChannels {
    #[init]
    pub fn new() -> Self {
        Self {
            channels: UnorderedMap::new(b"c"),
            next_id: 0,
        }
    }

    #[payable]
    pub fn create_channel(&mut self, receiver: AccountId, expiration: u64) -> u64 {
        let sender = env::predecessor_account_id();
        let deposit = env::attached_deposit();

        assert_ne!(sender, receiver, "Cannot create channel with self");
        assert!(deposit > 0, "Deposit required");
        assert!(expiration > env::block_timestamp(), "Invalid expiration");

        let channel_id = self.next_id;
        self.next_id += 1;

        let channel = Channel {
            sender,
            receiver,
            balance: deposit,
            total_claimed: 0,
            expiration,
            status: ChannelStatus::Open,
            created_at: env::block_timestamp(),
        };

        self.channels.insert(&channel_id, &channel);
        channel_id
    }

    #[payable]
    pub fn add_funds(&mut self, channel_id: u64) {
        let sender = env::predecessor_account_id();
        let deposit = env::attached_deposit();

        let mut channel = self.channels.get(&channel_id).expect("Channel not found");
        assert_eq!(channel.sender, sender, "Not authorized");
        assert_eq!(channel.status, ChannelStatus::Open, "Channel closed");
        assert!(deposit > 0, "Deposit required");

        channel.balance += deposit;
        self.channels.insert(&channel_id, &channel);
    }

    pub fn claim_funds(&mut self, channel_id: u64, amount: Balance) -> Promise {
        let receiver = env::predecessor_account_id();

        let mut channel = self.channels.get(&channel_id).expect("Channel not found");
        assert_eq!(channel.receiver, receiver, "Not authorized");
        assert_eq!(channel.status, ChannelStatus::Open, "Channel closed");

        let available = channel.balance - channel.total_claimed;
        assert!(amount <= available, "Insufficient funds");

        channel.total_claimed += amount;
        self.channels.insert(&channel_id, &channel);

        Promise::new(receiver).transfer(amount)
    }

    pub fn close_channel(&mut self, channel_id: u64) -> Promise {
        let caller = env::predecessor_account_id();

        let mut channel = self.channels.get(&channel_id).expect("Channel not found");
        assert_eq!(channel.status, ChannelStatus::Open, "Channel already closed");

        if channel.sender == caller {
            assert!(env::block_timestamp() >= channel.expiration, "Not expired");
        } else {
            assert_eq!(channel.receiver, caller, "Not authorized");
        }

        channel.status = ChannelStatus::Closed;
        self.channels.insert(&channel_id, &channel);

        let remaining = channel.balance - channel.total_claimed;
        if remaining > 0 {
            Promise::new(channel.sender).transfer(remaining)
        } else {
            Promise::new(caller).transfer(0)
        }
    }

    pub fn get_channel(&self, channel_id: u64) -> Option<Channel> {
        self.channels.get(&channel_id)
    }

    pub fn get_available_balance(&self, channel_id: u64) -> Balance {
        let channel = self.channels.get(&channel_id).expect("Channel not found");
        channel.balance - channel.total_claimed
    }
}
