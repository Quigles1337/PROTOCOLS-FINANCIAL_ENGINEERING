#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod payment_channels {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct PaymentChannels {
        admin: AccountId,
        channels: Mapping<u64, Channel>,
        channel_counter: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq))]
    pub struct Channel {
        pub id: u64,
        pub participant_a: AccountId,
        pub participant_b: AccountId,
        pub balance_a: Balance,
        pub balance_b: Balance,
        pub nonce: u64,
        pub status: ChannelStatus,
        pub expiration: u64,
        pub dispute_expiration: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq))]
    pub enum ChannelStatus {
        Open,
        InDispute,
        Closed,
    }

    #[ink(event)]
    pub struct ChannelOpened {
        #[ink(topic)]
        channel_id: u64,
        participant_a: AccountId,
        participant_b: AccountId,
        balance_a: Balance,
        balance_b: Balance,
    }

    #[ink(event)]
    pub struct ChannelUpdated {
        #[ink(topic)]
        channel_id: u64,
        nonce: u64,
        balance_a: Balance,
        balance_b: Balance,
    }

    #[ink(event)]
    pub struct DisputeRaised {
        #[ink(topic)]
        channel_id: u64,
        raised_by: AccountId,
    }

    #[ink(event)]
    pub struct ChannelClosed {
        #[ink(topic)]
        channel_id: u64,
    }

    impl PaymentChannels {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                channels: Mapping::new(),
                channel_counter: 0,
            }
        }

        #[ink(message, payable)]
        pub fn open_channel(&mut self, participant_b: AccountId, expiration: u64) -> u64 {
            let participant_a = self.env().caller();
            let balance_a = self.env().transferred_value();

            assert!(participant_a != participant_b, "Cannot create channel with self");
            assert!(balance_a > 0, "Must deposit funds");

            self.channel_counter += 1;
            let channel_id = self.channel_counter;

            let channel = Channel {
                id: channel_id,
                participant_a,
                participant_b,
                balance_a,
                balance_b: 0,
                nonce: 0,
                status: ChannelStatus::Open,
                expiration,
                dispute_expiration: 0,
            };

            self.channels.insert(channel_id, &channel);

            self.env().emit_event(ChannelOpened {
                channel_id,
                participant_a,
                participant_b,
                balance_a,
                balance_b: 0,
            });

            channel_id
        }

        #[ink(message, payable)]
        pub fn fund_channel(&mut self, channel_id: u64) {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();
            let mut channel = self.channels.get(channel_id).expect("Channel not found");

            assert!(matches!(channel.status, ChannelStatus::Open), "Channel not open");
            assert!(caller == channel.participant_b, "Only participant B can fund");

            channel.balance_b += amount;
            self.channels.insert(channel_id, &channel);
        }

        #[ink(message)]
        pub fn cooperative_close(&mut self, channel_id: u64, final_balance_a: Balance, final_balance_b: Balance) {
            let caller = self.env().caller();
            let mut channel = self.channels.get(channel_id).expect("Channel not found");

            assert!(matches!(channel.status, ChannelStatus::Open), "Channel not open");
            assert!(
                caller == channel.participant_a || caller == channel.participant_b,
                "Not a participant"
            );
            assert!(
                final_balance_a + final_balance_b == channel.balance_a + channel.balance_b,
                "Balances must sum correctly"
            );

            channel.status = ChannelStatus::Closed;
            self.channels.insert(channel_id, &channel);

            if final_balance_a > 0 {
                self.env().transfer(channel.participant_a, final_balance_a).expect("Transfer failed");
            }
            if final_balance_b > 0 {
                self.env().transfer(channel.participant_b, final_balance_b).expect("Transfer failed");
            }

            self.env().emit_event(ChannelClosed { channel_id });
        }

        #[ink(message)]
        pub fn raise_dispute(&mut self, channel_id: u64, nonce: u64, balance_a: Balance, balance_b: Balance) {
            let caller = self.env().caller();
            let mut channel = self.channels.get(channel_id).expect("Channel not found");

            assert!(matches!(channel.status, ChannelStatus::Open), "Channel not open");
            assert!(
                caller == channel.participant_a || caller == channel.participant_b,
                "Not a participant"
            );
            assert!(nonce > channel.nonce, "Nonce must be higher");
            assert!(
                balance_a + balance_b == channel.balance_a + channel.balance_b,
                "Balances must sum correctly"
            );

            channel.status = ChannelStatus::InDispute;
            channel.nonce = nonce;
            channel.balance_a = balance_a;
            channel.balance_b = balance_b;
            channel.dispute_expiration = self.env().block_number() + 100;
            self.channels.insert(channel_id, &channel);

            self.env().emit_event(DisputeRaised {
                channel_id,
                raised_by: caller,
            });
        }

        #[ink(message)]
        pub fn settle_dispute(&mut self, channel_id: u64) {
            let channel = self.channels.get(channel_id).expect("Channel not found");

            assert!(matches!(channel.status, ChannelStatus::InDispute), "No dispute");
            assert!(
                self.env().block_number() >= channel.dispute_expiration,
                "Dispute period not over"
            );

            let mut updated_channel = channel;
            updated_channel.status = ChannelStatus::Closed;
            self.channels.insert(channel_id, &updated_channel);

            if updated_channel.balance_a > 0 {
                self.env().transfer(updated_channel.participant_a, updated_channel.balance_a).expect("Transfer failed");
            }
            if updated_channel.balance_b > 0 {
                self.env().transfer(updated_channel.participant_b, updated_channel.balance_b).expect("Transfer failed");
            }

            self.env().emit_event(ChannelClosed { channel_id });
        }

        #[ink(message)]
        pub fn get_channel(&self, channel_id: u64) -> Option<Channel> {
            self.channels.get(channel_id)
        }

        #[ink(message)]
        pub fn get_channel_count(&self) -> u64 {
            self.channel_counter
        }
    }
}
