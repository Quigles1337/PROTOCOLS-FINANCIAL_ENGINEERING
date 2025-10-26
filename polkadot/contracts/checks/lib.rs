#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod checks {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct Checks {
        admin: AccountId,
        checks: Mapping<u64, Check>,
        check_counter: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct Check {
        pub id: u64,
        pub drawer: AccountId,
        pub payee: AccountId,
        pub amount: Balance,
        pub expiration: u64,
        pub status: CheckStatus,
        pub memo: [u8; 32],
        pub created_at: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub enum CheckStatus {
        Active,
        Cashed,
        Cancelled,
        Expired,
    }

    #[ink(event)]
    pub struct CheckCreated {
        #[ink(topic)]
        check_id: u64,
        drawer: AccountId,
        payee: AccountId,
        amount: Balance,
        expiration: u64,
    }

    #[ink(event)]
    pub struct CheckCashed {
        #[ink(topic)]
        check_id: u64,
        cashed_by: AccountId,
    }

    #[ink(event)]
    pub struct CheckCancelled {
        #[ink(topic)]
        check_id: u64,
    }

    impl Checks {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                checks: Mapping::new(),
                check_counter: 0,
            }
        }

        #[ink(message, payable)]
        pub fn create_check(
            &mut self,
            payee: AccountId,
            expiration: u64,
            memo: [u8; 32],
        ) -> u64 {
            let drawer = self.env().caller();
            let amount = self.env().transferred_value();
            let current_block = self.env().block_number();

            assert!(amount > 0, "Must deposit funds");
            assert!(expiration > current_block, "Expiration must be in future");
            assert!(drawer != payee, "Cannot create check to self");

            self.check_counter += 1;
            let check_id = self.check_counter;

            let check = Check {
                id: check_id,
                drawer,
                payee,
                amount,
                expiration,
                status: CheckStatus::Active,
                memo,
                created_at: current_block,
            };

            self.checks.insert(check_id, &check);

            self.env().emit_event(CheckCreated {
                check_id,
                drawer,
                payee,
                amount,
                expiration,
            });

            check_id
        }

        #[ink(message)]
        pub fn cash_check(&mut self, check_id: u64) {
            let caller = self.env().caller();
            let current_block = self.env().block_number();
            let mut check = self.checks.get(check_id).expect("Check not found");

            assert!(matches!(check.status, CheckStatus::Active), "Check not active");
            assert!(caller == check.payee, "Only payee can cash check");
            assert!(current_block < check.expiration, "Check expired");

            check.status = CheckStatus::Cashed;
            self.checks.insert(check_id, &check);

            self.env()
                .transfer(check.payee, check.amount)
                .expect("Transfer failed");

            self.env().emit_event(CheckCashed {
                check_id,
                cashed_by: caller,
            });
        }

        #[ink(message)]
        pub fn cancel_check(&mut self, check_id: u64) {
            let caller = self.env().caller();
            let mut check = self.checks.get(check_id).expect("Check not found");

            assert!(matches!(check.status, CheckStatus::Active), "Check not active");
            assert!(caller == check.drawer, "Only drawer can cancel");

            check.status = CheckStatus::Cancelled;
            self.checks.insert(check_id, &check);

            self.env()
                .transfer(check.drawer, check.amount)
                .expect("Refund failed");

            self.env().emit_event(CheckCancelled { check_id });
        }

        #[ink(message)]
        pub fn expire_check(&mut self, check_id: u64) {
            let current_block = self.env().block_number();
            let mut check = self.checks.get(check_id).expect("Check not found");

            assert!(matches!(check.status, CheckStatus::Active), "Check not active");
            assert!(current_block >= check.expiration, "Check not expired yet");

            check.status = CheckStatus::Expired;
            self.checks.insert(check_id, &check);

            self.env()
                .transfer(check.drawer, check.amount)
                .expect("Refund failed");
        }

        #[ink(message)]
        pub fn endorse_check(&mut self, check_id: u64, new_payee: AccountId) {
            let caller = self.env().caller();
            let mut check = self.checks.get(check_id).expect("Check not found");

            assert!(matches!(check.status, CheckStatus::Active), "Check not active");
            assert!(caller == check.payee, "Only current payee can endorse");
            assert!(new_payee != check.drawer, "Cannot endorse back to drawer");

            check.payee = new_payee;
            self.checks.insert(check_id, &check);
        }

        #[ink(message)]
        pub fn get_check(&self, check_id: u64) -> Option<Check> {
            self.checks.get(check_id)
        }

        #[ink(message)]
        pub fn get_check_count(&self) -> u64 {
            self.check_counter
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_create_check() {
            let mut contract = Checks::new();
            let check_id = contract.create_check(
                AccountId::from([0x01; 32]),
                1000,
                [0u8; 32],
            );
            assert_eq!(check_id, 1);
        }
    }
}
