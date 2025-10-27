use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SignerInfo {
    pub signer: AccountId,
    pub weight: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SignerList {
    pub owner: AccountId,
    pub signers: Vec<SignerInfo>,
    pub quorum: u64,
    pub active: bool,
    pub created_at: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum ProposalStatus {
    Pending,
    Executed,
    Rejected,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Proposal {
    pub list_id: u64,
    pub proposer: AccountId,
    pub description: String,
    pub approvals: Vec<AccountId>,
    pub approval_weight: u64,
    pub status: ProposalStatus,
    pub created_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct SignerListContract {
    signer_lists: UnorderedMap<u64, SignerList>,
    proposals: UnorderedMap<u64, Proposal>,
    list_signers: UnorderedMap<u64, UnorderedMap<AccountId, u64>>,
    next_list_id: u64,
    next_proposal_id: u64,
}

#[near_bindgen]
impl SignerListContract {
    #[init]
    pub fn new() -> Self {
        Self {
            signer_lists: UnorderedMap::new(b"l"),
            proposals: UnorderedMap::new(b"p"),
            list_signers: UnorderedMap::new(b"s"),
            next_list_id: 0,
            next_proposal_id: 0,
        }
    }

    pub fn create_signer_list(&mut self, quorum: u64) -> u64 {
        let owner = env::predecessor_account_id();

        assert!(quorum > 0 && quorum <= 10000, "Quorum must be 1-10000 (0.01%-100%)");

        let list_id = self.next_list_id;
        self.next_list_id += 1;

        let signer_list = SignerList {
            owner,
            signers: Vec::new(),
            quorum,
            active: true,
            created_at: env::block_timestamp(),
        };

        self.signer_lists.insert(&list_id, &signer_list);

        let signers_map = UnorderedMap::new(format!("s{}", list_id).as_bytes());
        self.list_signers.insert(&list_id, &signers_map);

        list_id
    }

    pub fn add_signer(&mut self, list_id: u64, new_signer: AccountId, weight: u64) {
        let owner = env::predecessor_account_id();

        let mut signer_list = self.signer_lists.get(&list_id).expect("List not found");
        assert_eq!(signer_list.owner, owner, "Not authorized");
        assert!(signer_list.active, "List not active");
        assert!(weight > 0 && weight <= 10000, "Weight must be 1-10000");

        let mut signers_map = self.list_signers.get(&list_id).expect("Signers map not found");
        assert!(!signers_map.get(&new_signer).is_some(), "Signer already exists");

        signers_map.insert(&new_signer, &weight);
        self.list_signers.insert(&list_id, &signers_map);

        signer_list.signers.push(SignerInfo {
            signer: new_signer,
            weight,
        });

        self.signer_lists.insert(&list_id, &signer_list);
    }

    pub fn remove_signer(&mut self, list_id: u64, signer: AccountId) {
        let owner = env::predecessor_account_id();

        let mut signer_list = self.signer_lists.get(&list_id).expect("List not found");
        assert_eq!(signer_list.owner, owner, "Not authorized");
        assert!(signer_list.active, "List not active");

        let mut signers_map = self.list_signers.get(&list_id).expect("Signers map not found");
        assert!(signers_map.get(&signer).is_some(), "Signer not found");

        signers_map.remove(&signer);
        self.list_signers.insert(&list_id, &signers_map);

        signer_list.signers.retain(|s| s.signer != signer);
        self.signer_lists.insert(&list_id, &signer_list);
    }

    pub fn create_proposal(&mut self, list_id: u64, description: String) -> u64 {
        let proposer = env::predecessor_account_id();

        let signer_list = self.signer_lists.get(&list_id).expect("List not found");
        assert!(signer_list.active, "List not active");

        let signers_map = self.list_signers.get(&list_id).expect("Signers map not found");
        assert!(signers_map.get(&proposer).is_some(), "Not a signer");

        let proposal_id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = Proposal {
            list_id,
            proposer,
            description,
            approvals: Vec::new(),
            approval_weight: 0,
            status: ProposalStatus::Pending,
            created_at: env::block_timestamp(),
        };

        self.proposals.insert(&proposal_id, &proposal);
        proposal_id
    }

    pub fn approve_proposal(&mut self, proposal_id: u64) {
        let approver = env::predecessor_account_id();

        let mut proposal = self.proposals.get(&proposal_id).expect("Proposal not found");
        assert_eq!(proposal.status, ProposalStatus::Pending, "Proposal not pending");

        let signers_map = self.list_signers.get(&proposal.list_id).expect("Signers map not found");
        let weight = signers_map.get(&approver).expect("Not a signer");

        assert!(!proposal.approvals.contains(&approver), "Already approved");

        proposal.approvals.push(approver);
        proposal.approval_weight += weight;

        self.proposals.insert(&proposal_id, &proposal);
    }

    pub fn execute_proposal(&mut self, proposal_id: u64) {
        let executor = env::predecessor_account_id();

        let mut proposal = self.proposals.get(&proposal_id).expect("Proposal not found");
        assert_eq!(proposal.status, ProposalStatus::Pending, "Proposal not pending");

        let signer_list = self.signer_lists.get(&proposal.list_id).expect("List not found");
        assert!(signer_list.active, "List not active");

        let signers_map = self.list_signers.get(&proposal.list_id).expect("Signers map not found");
        assert!(signers_map.get(&executor).is_some(), "Not a signer");

        assert!(
            proposal.approval_weight >= signer_list.quorum,
            "Quorum not met"
        );

        proposal.status = ProposalStatus::Executed;
        self.proposals.insert(&proposal_id, &proposal);
    }

    pub fn get_signer_list(&self, list_id: u64) -> Option<SignerList> {
        self.signer_lists.get(&list_id)
    }

    pub fn get_proposal(&self, proposal_id: u64) -> Option<Proposal> {
        self.proposals.get(&proposal_id)
    }

    pub fn get_signer_weight(&self, list_id: u64, signer: AccountId) -> u64 {
        if let Some(signers_map) = self.list_signers.get(&list_id) {
            signers_map.get(&signer).unwrap_or(0)
        } else {
            0
        }
    }

    pub fn has_quorum(&self, proposal_id: u64) -> bool {
        if let Some(proposal) = self.proposals.get(&proposal_id) {
            if let Some(signer_list) = self.signer_lists.get(&proposal.list_id) {
                proposal.approval_weight >= signer_list.quorum
            } else {
                false
            }
        } else {
            false
        }
    }
}
