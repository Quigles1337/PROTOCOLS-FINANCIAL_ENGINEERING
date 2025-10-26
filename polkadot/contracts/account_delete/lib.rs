#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod account_delete {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct AccountDelete {
        admin: AccountId,
        accounts: Mapping<AccountId, AccountInfo>,
        deletion_requests: Mapping<AccountId, DeletionRequest>,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct AccountInfo {
        pub owner: AccountId,
        pub balance: Balance,
        pub beneficiary: Option<AccountId>,
        pub active: bool,
        pub created_at: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct DeletionRequest {
        pub owner: AccountId,
        pub beneficiary: AccountId,
        pub requested_at: u64,
        pub grace_period_end: u64,
        pub executed: bool,
    }

    #[ink(event)]
    pub struct AccountCreated {
        #[ink(topic)]
        owner: AccountId,
    }

    #[ink(event)]
    pub struct BeneficiarySet {
        #[ink(topic)]
        owner: AccountId,
        beneficiary: AccountId,
    }

    #[ink(event)]
    pub struct DeletionRequested {
        #[ink(topic)]
        owner: AccountId,
        beneficiary: AccountId,
        grace_period_end: u64,
    }

    #[ink(event)]
    pub struct AccountDeleted {
        #[ink(topic)]
        owner: AccountId,
        beneficiary: AccountId,
        balance: Balance,
    }

    impl AccountDelete {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                accounts: Mapping::new(),
                deletion_requests: Mapping::new(),
            }
        }

        #[ink(message, payable)]
        pub fn create_account(&mut self) {
            let owner = self.env().caller();
            let balance = self.env().transferred_value();
            let current_block = self.env().block_number();

            assert!(!self.accounts.contains(owner), "Account already exists");

            let account = AccountInfo {
                owner,
                balance,
                beneficiary: None,
                active: true,
                created_at: current_block,
            };

            self.accounts.insert(owner, &account);

            self.env().emit_event(AccountCreated { owner });
        }

        #[ink(message)]
        pub fn set_beneficiary(&mut self, beneficiary: AccountId) {
            let owner = self.env().caller();
            let mut account = self.accounts.get(owner).expect("Account not found");

            assert!(account.active, "Account not active");
            assert!(beneficiary != owner, "Cannot set self as beneficiary");

            account.beneficiary = Some(beneficiary);
            self.accounts.insert(owner, &account);

            self.env().emit_event(BeneficiarySet {
                owner,
                beneficiary,
            });
        }

        #[ink(message)]
        pub fn request_deletion(&mut self, beneficiary: AccountId) {
            let owner = self.env().caller();
            let current_block = self.env().block_number();
            let account = self.accounts.get(owner).expect("Account not found");

            assert!(account.active, "Account not active");
            assert!(!self.deletion_requests.contains(owner), "Deletion already requested");

            let grace_period = 100u64;
            let grace_period_end = current_block + grace_period;

            let request = DeletionRequest {
                owner,
                beneficiary,
                requested_at: current_block,
                grace_period_end,
                executed: false,
            };

            self.deletion_requests.insert(owner, &request);

            self.env().emit_event(DeletionRequested {
                owner,
                beneficiary,
                grace_period_end,
            });
        }

        #[ink(message)]
        pub fn execute_deletion(&mut self) {
            let owner = self.env().caller();
            let current_block = self.env().block_number();
            let mut request = self.deletion_requests.get(owner).expect("No deletion request found");
            let mut account = self.accounts.get(owner).expect("Account not found");

            assert!(!request.executed, "Deletion already executed");
            assert!(current_block >= request.grace_period_end, "Grace period not ended");
            assert!(account.active, "Account not active");

            request.executed = true;
            account.active = false;

            let balance = account.balance;

            self.deletion_requests.insert(owner, &request);
            self.accounts.insert(owner, &account);

            if balance > 0 {
                self.env()
                    .transfer(request.beneficiary, balance)
                    .expect("Transfer failed");
            }

            self.env().emit_event(AccountDeleted {
                owner,
                beneficiary: request.beneficiary,
                balance,
            });
        }

        #[ink(message)]
        pub fn cancel_deletion(&mut self) {
            let owner = self.env().caller();
            let request = self.deletion_requests.get(owner).expect("No deletion request found");

            assert!(!request.executed, "Deletion already executed");

            self.deletion_requests.remove(owner);
        }

        #[ink(message, payable)]
        pub fn deposit(&mut self) {
            let owner = self.env().caller();
            let amount = self.env().transferred_value();
            let mut account = self.accounts.get(owner).expect("Account not found");

            assert!(account.active, "Account not active");

            account.balance += amount;
            self.accounts.insert(owner, &account);
        }

        #[ink(message)]
        pub fn get_account(&self, owner: AccountId) -> Option<AccountInfo> {
            self.accounts.get(owner)
        }

        #[ink(message)]
        pub fn get_deletion_request(&self, owner: AccountId) -> Option<DeletionRequest> {
            self.deletion_requests.get(owner)
        }
    }
}
