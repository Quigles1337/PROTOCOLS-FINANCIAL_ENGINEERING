#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! TrustLines - Bilateral Credit Networks with Payment Rippling
//! ink! 4.0 implementation for Polkadot/Substrate

#[ink::contract]
mod trust_lines {
    use ink::storage::Mapping;
    use ink::prelude::vec::Vec;

    /// Trust line structure
    #[derive(scale::Decode, scale::Encode, Clone, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TrustLine {
        /// Account 1 (lower address)
        account1: AccountId,
        /// Account 2 (higher address)
        account2: AccountId,
        /// Credit limit from account1 to account2
        limit1: Balance,
        /// Credit limit from account2 to account1
        limit2: Balance,
        /// Current balance (positive = account2 owes account1)
        balance: i128,
        /// Allow rippling
        allow_rippling: bool,
        /// Quality in (scaled by 1000)
        quality_in: u32,
        /// Quality out (scaled by 1000)
        quality_out: u32,
    }

    /// Storage for trust lines contract
    #[ink(storage)]
    pub struct TrustLines {
        /// Trust lines mapping: (account1, account2) => TrustLine
        trust_lines: Mapping<(AccountId, AccountId), TrustLine>,
        /// Owner of the contract
        owner: AccountId,
    }

    /// Events
    #[ink(event)]
    pub struct TrustLineCreated {
        #[ink(topic)]
        account1: AccountId,
        #[ink(topic)]
        account2: AccountId,
        limit1: Balance,
        limit2: Balance,
    }

    #[ink(event)]
    pub struct TrustLineUpdated {
        #[ink(topic)]
        account1: AccountId,
        #[ink(topic)]
        account2: AccountId,
        new_limit: Balance,
    }

    #[ink(event)]
    pub struct PaymentSent {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        amount: Balance,
        new_balance: i128,
    }

    /// Errors
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Trust line not found
        TrustLineNotFound,
        /// Trust line already exists
        TrustLineExists,
        /// Cannot create trust line with self
        SelfTrustLine,
        /// Limit must be greater than zero
        InvalidLimit,
        /// Insufficient credit
        InsufficientCredit,
        /// Invalid amount
        InvalidAmount,
        /// Unauthorized
        Unauthorized,
        /// Rippling not enabled
        RipplingDisabled,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl TrustLines {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                trust_lines: Mapping::default(),
                owner: Self::env().caller(),
            }
        }

        /// Create a new trust line
        #[ink(message)]
        pub fn create_trust_line(
            &mut self,
            counterparty: AccountId,
            limit: Balance,
            allow_rippling: bool,
        ) -> Result<()> {
            let caller = self.env().caller();

            // Cannot create trust line with self
            if caller == counterparty {
                return Err(Error::SelfTrustLine);
            }

            // Ensure accounts are ordered
            let (account1, account2, limit1, limit2) = if caller < counterparty {
                (caller, counterparty, limit, 0)
            } else {
                (counterparty, caller, 0, limit)
            };

            // Check if trust line already exists
            if self.trust_lines.contains(&(account1, account2)) {
                return Err(Error::TrustLineExists);
            }

            let trust_line = TrustLine {
                account1,
                account2,
                limit1,
                limit2,
                balance: 0,
                allow_rippling,
                quality_in: 1000,
                quality_out: 1000,
            };

            self.trust_lines.insert((account1, account2), &trust_line);

            self.env().emit_event(TrustLineCreated {
                account1,
                account2,
                limit1,
                limit2,
            });

            Ok(())
        }

        /// Update trust line limit
        #[ink(message)]
        pub fn update_limit(&mut self, counterparty: AccountId, new_limit: Balance) -> Result<()> {
            let caller = self.env().caller();
            let (account1, account2) = Self::order_accounts(caller, counterparty);

            let mut trust_line = self
                .trust_lines
                .get(&(account1, account2))
                .ok_or(Error::TrustLineNotFound)?;

            // Update the appropriate limit
            if caller == account1 {
                trust_line.limit1 = new_limit;
            } else {
                trust_line.limit2 = new_limit;
            }

            self.trust_lines.insert((account1, account2), &trust_line);

            self.env().emit_event(TrustLineUpdated {
                account1,
                account2,
                new_limit,
            });

            Ok(())
        }

        /// Send payment through trust line
        #[ink(message)]
        pub fn send_payment(&mut self, recipient: AccountId, amount: Balance) -> Result<()> {
            let caller = self.env().caller();

            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let (account1, account2) = Self::order_accounts(caller, recipient);

            let mut trust_line = self
                .trust_lines
                .get(&(account1, account2))
                .ok_or(Error::TrustLineNotFound)?;

            // Calculate new balance
            let amount_i128 = amount as i128;
            let new_balance = if caller == account1 {
                trust_line.balance - amount_i128
            } else {
                trust_line.balance + amount_i128
            };

            // Check credit limits
            if caller == account1 {
                let max_negative = -(trust_line.limit1 as i128);
                if new_balance < max_negative {
                    return Err(Error::InsufficientCredit);
                }
            } else {
                let max_positive = trust_line.limit2 as i128;
                if new_balance > max_positive {
                    return Err(Error::InsufficientCredit);
                }
            }

            trust_line.balance = new_balance;
            self.trust_lines.insert((account1, account2), &trust_line);

            self.env().emit_event(PaymentSent {
                from: caller,
                to: recipient,
                amount,
                new_balance,
            });

            Ok(())
        }

        /// Close trust line (must have zero balance)
        #[ink(message)]
        pub fn close_trust_line(&mut self, counterparty: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let (account1, account2) = Self::order_accounts(caller, counterparty);

            let trust_line = self
                .trust_lines
                .get(&(account1, account2))
                .ok_or(Error::TrustLineNotFound)?;

            // Can only close if balance is zero
            if trust_line.balance != 0 {
                return Err(Error::InsufficientCredit);
            }

            self.trust_lines.remove(&(account1, account2));

            Ok(())
        }

        /// Get trust line details
        #[ink(message)]
        pub fn get_trust_line(&self, account1: AccountId, account2: AccountId) -> Option<TrustLine> {
            let (acc1, acc2) = Self::order_accounts(account1, account2);
            self.trust_lines.get(&(acc1, acc2))
        }

        /// Get available credit
        #[ink(message)]
        pub fn get_available_credit(&self, from: AccountId, to: AccountId) -> Balance {
            let (account1, account2) = Self::order_accounts(from, to);

            if let Some(trust_line) = self.trust_lines.get(&(account1, account2)) {
                if from == account1 {
                    let used = if trust_line.balance < 0 {
                        (-trust_line.balance) as u128
                    } else {
                        0
                    };
                    trust_line.limit1.saturating_sub(used)
                } else {
                    let used = if trust_line.balance > 0 {
                        trust_line.balance as u128
                    } else {
                        0
                    };
                    trust_line.limit2.saturating_sub(used)
                }
            } else {
                0
            }
        }

        /// Helper: Order accounts consistently
        fn order_accounts(account1: AccountId, account2: AccountId) -> (AccountId, AccountId) {
            if account1 < account2 {
                (account1, account2)
            } else {
                (account2, account1)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn create_trust_line_works() {
            let mut contract = TrustLines::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(
                contract.create_trust_line(accounts.bob, 1000, true),
                Ok(())
            );
        }

        #[ink::test]
        fn send_payment_works() {
            let mut contract = TrustLines::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            contract.create_trust_line(accounts.bob, 1000, true).unwrap();
            assert_eq!(contract.send_payment(accounts.bob, 100), Ok(()));
        }

        #[ink::test]
        fn insufficient_credit_fails() {
            let mut contract = TrustLines::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            contract.create_trust_line(accounts.bob, 100, true).unwrap();
            assert_eq!(
                contract.send_payment(accounts.bob, 200),
                Err(Error::InsufficientCredit)
            );
        }
    }
}
