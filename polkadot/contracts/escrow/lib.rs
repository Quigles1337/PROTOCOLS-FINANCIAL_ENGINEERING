#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod escrow {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct Escrow {
        admin: AccountId,
        escrows: Mapping<u64, EscrowData>,
        escrow_counter: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq))]
    pub struct EscrowData {
        pub id: u64,
        pub sender: AccountId,
        pub recipient: AccountId,
        pub amount: Balance,
        pub hash_lock: [u8; 32],
        pub time_lock: u64,
        pub status: EscrowStatus,
        pub created_at: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq))]
    pub enum EscrowStatus {
        Active,
        Completed,
        Refunded,
    }

    #[ink(event)]
    pub struct EscrowCreated {
        #[ink(topic)]
        escrow_id: u64,
        sender: AccountId,
        recipient: AccountId,
        amount: Balance,
        time_lock: u64,
    }

    #[ink(event)]
    pub struct EscrowCompleted {
        #[ink(topic)]
        escrow_id: u64,
        preimage: [u8; 32],
    }

    #[ink(event)]
    pub struct EscrowRefunded {
        #[ink(topic)]
        escrow_id: u64,
    }

    impl Escrow {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                escrows: Mapping::new(),
                escrow_counter: 0,
            }
        }

        #[ink(message, payable)]
        pub fn create_escrow(
            &mut self,
            recipient: AccountId,
            hash_lock: [u8; 32],
            time_lock: u64,
        ) -> u64 {
            let sender = self.env().caller();
            let amount = self.env().transferred_value();
            let current_block = self.env().block_number();

            assert!(amount > 0, "Must deposit funds");
            assert!(time_lock > current_block, "Time lock must be in future");

            self.escrow_counter += 1;
            let escrow_id = self.escrow_counter;

            let escrow = EscrowData {
                id: escrow_id,
                sender,
                recipient,
                amount,
                hash_lock,
                time_lock,
                status: EscrowStatus::Active,
                created_at: current_block,
            };

            self.escrows.insert(escrow_id, &escrow);

            self.env().emit_event(EscrowCreated {
                escrow_id,
                sender,
                recipient,
                amount,
                time_lock,
            });

            escrow_id
        }

        #[ink(message)]
        pub fn complete_escrow(&mut self, escrow_id: u64, preimage: [u8; 32]) {
            let caller = self.env().caller();
            let mut escrow = self.escrows.get(escrow_id).expect("Escrow not found");

            assert!(matches!(escrow.status, EscrowStatus::Active), "Escrow not active");
            assert!(caller == escrow.recipient, "Only recipient can complete");

            let mut hash_input = [0u8; 32];
            hash_input.copy_from_slice(&preimage);
            let computed_hash = self.hash_preimage(&hash_input);

            assert!(computed_hash == escrow.hash_lock, "Invalid preimage");

            escrow.status = EscrowStatus::Completed;
            self.escrows.insert(escrow_id, &escrow);

            self.env()
                .transfer(escrow.recipient, escrow.amount)
                .expect("Transfer failed");

            self.env().emit_event(EscrowCompleted {
                escrow_id,
                preimage,
            });
        }

        #[ink(message)]
        pub fn refund_escrow(&mut self, escrow_id: u64) {
            let caller = self.env().caller();
            let current_block = self.env().block_number();
            let mut escrow = self.escrows.get(escrow_id).expect("Escrow not found");

            assert!(matches!(escrow.status, EscrowStatus::Active), "Escrow not active");
            assert!(caller == escrow.sender, "Only sender can refund");
            assert!(current_block >= escrow.time_lock, "Time lock not expired");

            escrow.status = EscrowStatus::Refunded;
            self.escrows.insert(escrow_id, &escrow);

            self.env()
                .transfer(escrow.sender, escrow.amount)
                .expect("Transfer failed");

            self.env().emit_event(EscrowRefunded { escrow_id });
        }

        fn hash_preimage(&self, preimage: &[u8; 32]) -> [u8; 32] {
            use ink::env::hash::{HashOutput, Sha2x256};
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            ink::env::hash_bytes::<Sha2x256>(preimage, &mut output);
            output
        }

        #[ink(message)]
        pub fn get_escrow(&self, escrow_id: u64) -> Option<EscrowData> {
            self.escrows.get(escrow_id)
        }

        #[ink(message)]
        pub fn get_escrow_count(&self) -> u64 {
            self.escrow_counter
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_create_escrow() {
            let mut contract = Escrow::new();
            let hash_lock = [1u8; 32];
            let escrow_id = contract.create_escrow(
                AccountId::from([0x01; 32]),
                hash_lock,
                1000,
            );
            assert_eq!(escrow_id, 1);
        }
    }
}
