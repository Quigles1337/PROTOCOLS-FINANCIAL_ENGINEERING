#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod deposit_preauth {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct DepositPreauth {
        admin: AccountId,
        preauths: Mapping<(AccountId, AccountId), PreauthData>,
        preauth_counter: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct PreauthData {
        pub authorizer: AccountId,
        pub authorized: AccountId,
        pub max_amount: Balance,
        pub used: bool,
        pub expiration: u64,
        pub created_at: u64,
    }

    #[ink(event)]
    pub struct PreauthCreated {
        #[ink(topic)]
        authorizer: AccountId,
        authorized: AccountId,
        max_amount: Balance,
    }

    #[ink(event)]
    pub struct PreauthUsed {
        #[ink(topic)]
        authorizer: AccountId,
        authorized: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct PreauthRevoked {
        #[ink(topic)]
        authorizer: AccountId,
        authorized: AccountId,
    }

    impl DepositPreauth {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                preauths: Mapping::new(),
                preauth_counter: 0,
            }
        }

        #[ink(message)]
        pub fn create_preauth(
            &mut self,
            authorized: AccountId,
            max_amount: Balance,
            expiration: u64,
        ) {
            let authorizer = self.env().caller();
            let current_block = self.env().block_number();

            assert!(max_amount > 0, "Max amount must be positive");
            assert!(expiration > current_block, "Expiration must be in future");
            assert!(authorizer != authorized, "Cannot preauth self");

            let preauth = PreauthData {
                authorizer,
                authorized,
                max_amount,
                used: false,
                expiration,
                created_at: current_block,
            };

            self.preauths.insert((authorizer, authorized), &preauth);
            self.preauth_counter += 1;

            self.env().emit_event(PreauthCreated {
                authorizer,
                authorized,
                max_amount,
            });
        }

        #[ink(message, payable)]
        pub fn use_preauth(&mut self, authorizer: AccountId) {
            let authorized = self.env().caller();
            let amount = self.env().transferred_value();
            let current_block = self.env().block_number();
            let key = (authorizer, authorized);

            let mut preauth = self.preauths.get(key).expect("Preauth not found");

            assert!(!preauth.used, "Preauth already used");
            assert!(current_block < preauth.expiration, "Preauth expired");
            assert!(amount <= preauth.max_amount, "Amount exceeds max");

            preauth.used = true;
            self.preauths.insert(key, &preauth);

            self.env()
                .transfer(authorizer, amount)
                .expect("Transfer failed");

            self.env().emit_event(PreauthUsed {
                authorizer,
                authorized,
                amount,
            });
        }

        #[ink(message)]
        pub fn revoke_preauth(&mut self, authorized: AccountId) {
            let authorizer = self.env().caller();
            let key = (authorizer, authorized);

            let mut preauth = self.preauths.get(key).expect("Preauth not found");
            assert!(!preauth.used, "Preauth already used");

            self.preauths.remove(key);

            self.env().emit_event(PreauthRevoked {
                authorizer,
                authorized,
            });
        }

        #[ink(message)]
        pub fn check_preauth(
            &self,
            authorizer: AccountId,
            authorized: AccountId,
        ) -> bool {
            if let Some(preauth) = self.preauths.get((authorizer, authorized)) {
                !preauth.used && self.env().block_number() < preauth.expiration
            } else {
                false
            }
        }

        #[ink(message)]
        pub fn get_preauth(
            &self,
            authorizer: AccountId,
            authorized: AccountId,
        ) -> Option<PreauthData> {
            self.preauths.get((authorizer, authorized))
        }

        #[ink(message)]
        pub fn get_preauth_count(&self) -> u64 {
            self.preauth_counter
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_create_preauth() {
            let mut contract = DepositPreauth::new();
            contract.create_preauth(
                AccountId::from([0x01; 32]),
                1000,
                100,
            );
            assert_eq!(contract.get_preauth_count(), 1);
        }
    }
}
