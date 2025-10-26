#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod signer_list {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct SignerList {
        admin: AccountId,
        signer_lists: Mapping<AccountId, Vec<Signer>>,
        proposals: Mapping<(AccountId, u64), Proposal>,
        proposal_counters: Mapping<AccountId, u64>,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct Signer {
        pub account: AccountId,
        pub weight: u32,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct Proposal {
        pub id: u64,
        pub proposer: AccountId,
        pub target: AccountId,
        pub amount: Balance,
        pub approvals: Vec<AccountId>,
        pub total_weight: u32,
        pub executed: bool,
        pub created_at: u64,
    }

    #[ink(event)]
    pub struct SignerAdded {
        #[ink(topic)]
        owner: AccountId,
        signer: AccountId,
        weight: u32,
    }

    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        owner: AccountId,
        proposal_id: u64,
        target: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct ProposalApproved {
        #[ink(topic)]
        owner: AccountId,
        proposal_id: u64,
        approver: AccountId,
    }

    #[ink(event)]
    pub struct ProposalExecuted {
        #[ink(topic)]
        owner: AccountId,
        proposal_id: u64,
    }

    impl SignerList {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                signer_lists: Mapping::new(),
                proposals: Mapping::new(),
                proposal_counters: Mapping::new(),
            }
        }

        #[ink(message)]
        pub fn add_signer(&mut self, signer: AccountId, weight: u32) {
            let owner = self.env().caller();
            assert!(weight > 0, "Weight must be positive");

            let mut signers = self.signer_lists.get(owner).unwrap_or_default();
            
            for s in &signers {
                assert!(s.account != signer, "Signer already exists");
            }

            signers.push(Signer {
                account: signer,
                weight,
            });

            self.signer_lists.insert(owner, &signers);

            self.env().emit_event(SignerAdded {
                owner,
                signer,
                weight,
            });
        }

        #[ink(message)]
        pub fn remove_signer(&mut self, signer: AccountId) {
            let owner = self.env().caller();
            let mut signers = self.signer_lists.get(owner).expect("No signers found");

            signers.retain(|s| s.account != signer);
            self.signer_lists.insert(owner, &signers);
        }

        #[ink(message, payable)]
        pub fn create_proposal(&mut self, target: AccountId, amount: Balance) -> u64 {
            let owner = self.env().caller();
            let deposit = self.env().transferred_value();
            let current_block = self.env().block_number();

            assert!(deposit >= amount, "Insufficient deposit");

            let mut counter = self.proposal_counters.get(owner).unwrap_or(0);
            counter += 1;
            self.proposal_counters.insert(owner, &counter);

            let proposal = Proposal {
                id: counter,
                proposer: owner,
                target,
                amount,
                approvals: vec![],
                total_weight: 0,
                executed: false,
                created_at: current_block,
            };

            self.proposals.insert((owner, counter), &proposal);

            self.env().emit_event(ProposalCreated {
                owner,
                proposal_id: counter,
                target,
                amount,
            });

            counter
        }

        #[ink(message)]
        pub fn approve_proposal(&mut self, owner: AccountId, proposal_id: u64) {
            let approver = self.env().caller();
            let key = (owner, proposal_id);
            let mut proposal = self.proposals.get(key).expect("Proposal not found");

            assert!(!proposal.executed, "Proposal already executed");

            for approval in &proposal.approvals {
                assert!(*approval != approver, "Already approved");
            }

            let signers = self.signer_lists.get(owner).expect("No signers found");
            let signer = signers.iter().find(|s| s.account == approver);

            assert!(signer.is_some(), "Not a signer");

            let weight = signer.unwrap().weight;
            proposal.approvals.push(approver);
            proposal.total_weight += weight;

            self.proposals.insert(key, &proposal);

            self.env().emit_event(ProposalApproved {
                owner,
                proposal_id,
                approver,
            });
        }

        #[ink(message)]
        pub fn execute_proposal(&mut self, owner: AccountId, proposal_id: u64, quorum: u32) {
            let key = (owner, proposal_id);
            let mut proposal = self.proposals.get(key).expect("Proposal not found");

            assert!(!proposal.executed, "Proposal already executed");
            assert!(proposal.total_weight >= quorum, "Quorum not met");

            proposal.executed = true;
            self.proposals.insert(key, &proposal);

            self.env()
                .transfer(proposal.target, proposal.amount)
                .expect("Transfer failed");

            self.env().emit_event(ProposalExecuted {
                owner,
                proposal_id,
            });
        }

        #[ink(message)]
        pub fn get_signers(&self, owner: AccountId) -> Vec<Signer> {
            self.signer_lists.get(owner).unwrap_or_default()
        }

        #[ink(message)]
        pub fn get_proposal(&self, owner: AccountId, proposal_id: u64) -> Option<Proposal> {
            self.proposals.get((owner, proposal_id))
        }

        #[ink(message)]
        pub fn calculate_total_weight(&self, owner: AccountId) -> u32 {
            let signers = self.signer_lists.get(owner).unwrap_or_default();
            signers.iter().map(|s| s.weight).sum()
        }
    }
}
