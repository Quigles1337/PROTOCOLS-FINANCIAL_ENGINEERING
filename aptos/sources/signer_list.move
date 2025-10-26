module xrpl_primitives::signer_list {
    use std::signer;
    use std::error;
    use std::vector;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use aptos_std::simple_map::{Self, SimpleMap};

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_LIST_NOT_FOUND: u64 = 3;
    const E_PROPOSAL_NOT_FOUND: u64 = 4;
    const E_UNAUTHORIZED: u64 = 5;
    const E_ALREADY_APPROVED: u64 = 6;
    const E_NOT_SIGNER: u64 = 7;
    const E_QUORUM_NOT_MET: u64 = 8;
    const E_ALREADY_EXECUTED: u64 = 9;
    const E_SIGNER_EXISTS: u64 = 10;
    const E_SIGNER_NOT_FOUND: u64 = 11;

    /// Signer entry with weight
    struct SignerEntry has store, copy, drop {
        address: address,
        weight: u64,
    }

    /// Signer list for weighted multisig
    struct SignerList has store {
        owner: address,
        signers: vector<SignerEntry>,
        signer_weights: SimpleMap<address, u64>,
        total_weight: u64,
        quorum: u64,
        created_at: u64,
    }

    /// Proposal for multisig execution
    struct Proposal has store {
        signer_list_id: u64,
        proposer: address,
        description: vector<u8>,
        approvals: SimpleMap<address, bool>,
        approvals_weight: u64,
        executed: bool,
        created_at: u64,
    }

    /// Global registry
    struct SignerListRegistry has key {
        signer_lists: Table<u64, SignerList>,
        proposals: Table<u64, Proposal>,
        next_list_id: u64,
        next_proposal_id: u64,
    }

    /// Events
    #[event]
    struct SignerListCreated has drop, store {
        list_id: u64,
        owner: address,
        quorum: u64,
        timestamp: u64,
    }

    #[event]
    struct SignerAdded has drop, store {
        list_id: u64,
        signer: address,
        weight: u64,
        timestamp: u64,
    }

    #[event]
    struct SignerRemoved has drop, store {
        list_id: u64,
        signer: address,
        timestamp: u64,
    }

    #[event]
    struct ProposalCreated has drop, store {
        proposal_id: u64,
        list_id: u64,
        proposer: address,
        timestamp: u64,
    }

    #[event]
    struct ProposalApproved has drop, store {
        proposal_id: u64,
        approver: address,
        weight: u64,
        total_weight: u64,
        timestamp: u64,
    }

    #[event]
    struct ProposalExecuted has drop, store {
        proposal_id: u64,
        timestamp: u64,
    }

    /// Initialize the registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<SignerListRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, SignerListRegistry {
            signer_lists: table::new(),
            proposals: table::new(),
            next_list_id: 0,
            next_proposal_id: 0,
        });
    }

    /// Create a new signer list
    public fun create_signer_list(
        owner: &signer,
        quorum: u64,
    ): u64 acquires SignerListRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<SignerListRegistry>(registry_addr);
        let list_id = registry.next_list_id;
        registry.next_list_id = list_id + 1;

        let signer_list = SignerList {
            owner: owner_addr,
            signers: vector::empty(),
            signer_weights: simple_map::create(),
            total_weight: 0,
            quorum,
            created_at: timestamp::now_seconds(),
        };

        table::add(&mut registry.signer_lists, list_id, signer_list);

        event::emit(SignerListCreated {
            list_id,
            owner: owner_addr,
            quorum,
            timestamp: timestamp::now_seconds(),
        });

        list_id
    }

    /// Add a signer to the list
    public entry fun add_signer(
        owner: &signer,
        list_id: u64,
        new_signer: address,
        weight: u64,
    ) acquires SignerListRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.signer_lists, list_id), error::not_found(E_LIST_NOT_FOUND));

        let signer_list = table::borrow_mut(&mut registry.signer_lists, list_id);
        assert!(signer_list.owner == owner_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(!simple_map::contains_key(&signer_list.signer_weights, &new_signer), error::already_exists(E_SIGNER_EXISTS));

        let entry = SignerEntry {
            address: new_signer,
            weight,
        };

        vector::push_back(&mut signer_list.signers, entry);
        simple_map::add(&mut signer_list.signer_weights, new_signer, weight);
        signer_list.total_weight = signer_list.total_weight + weight;

        event::emit(SignerAdded {
            list_id,
            signer: new_signer,
            weight,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Remove a signer from the list
    public entry fun remove_signer(
        owner: &signer,
        list_id: u64,
        signer_to_remove: address,
    ) acquires SignerListRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.signer_lists, list_id), error::not_found(E_LIST_NOT_FOUND));

        let signer_list = table::borrow_mut(&mut registry.signer_lists, list_id);
        assert!(signer_list.owner == owner_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(simple_map::contains_key(&signer_list.signer_weights, &signer_to_remove), error::not_found(E_SIGNER_NOT_FOUND));

        let (_, weight) = simple_map::remove(&mut signer_list.signer_weights, &signer_to_remove);
        signer_list.total_weight = signer_list.total_weight - weight;

        // Remove from vector
        let len = vector::length(&signer_list.signers);
        let i = 0;
        while (i < len) {
            let entry = vector::borrow(&signer_list.signers, i);
            if (entry.address == signer_to_remove) {
                vector::remove(&mut signer_list.signers, i);
                break
            };
            i = i + 1;
        };

        event::emit(SignerRemoved {
            list_id,
            signer: signer_to_remove,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Create a proposal
    public fun create_proposal(
        proposer: &signer,
        list_id: u64,
        description: vector<u8>,
    ): u64 acquires SignerListRegistry {
        let proposer_addr = signer::address_of(proposer);

        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.signer_lists, list_id), error::not_found(E_LIST_NOT_FOUND));

        let signer_list = table::borrow(&registry.signer_lists, list_id);
        assert!(simple_map::contains_key(&signer_list.signer_weights, &proposer_addr), error::permission_denied(E_NOT_SIGNER));

        let proposal_id = registry.next_proposal_id;
        registry.next_proposal_id = proposal_id + 1;

        let proposal = Proposal {
            signer_list_id: list_id,
            proposer: proposer_addr,
            description,
            approvals: simple_map::create(),
            approvals_weight: 0,
            executed: false,
            created_at: timestamp::now_seconds(),
        };

        table::add(&mut registry.proposals, proposal_id, proposal);

        event::emit(ProposalCreated {
            proposal_id,
            list_id,
            proposer: proposer_addr,
            timestamp: timestamp::now_seconds(),
        });

        proposal_id
    }

    /// Approve a proposal
    public entry fun approve_proposal(
        approver: &signer,
        proposal_id: u64,
    ) acquires SignerListRegistry {
        let approver_addr = signer::address_of(approver);

        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.proposals, proposal_id), error::not_found(E_PROPOSAL_NOT_FOUND));

        let proposal = table::borrow_mut(&mut registry.proposals, proposal_id);
        assert!(!proposal.executed, error::invalid_state(E_ALREADY_EXECUTED));
        assert!(!simple_map::contains_key(&proposal.approvals, &approver_addr), error::already_exists(E_ALREADY_APPROVED));

        let signer_list = table::borrow(&registry.signer_lists, proposal.signer_list_id);
        assert!(simple_map::contains_key(&signer_list.signer_weights, &approver_addr), error::permission_denied(E_NOT_SIGNER));

        let weight = *simple_map::borrow(&signer_list.signer_weights, &approver_addr);
        simple_map::add(&mut proposal.approvals, approver_addr, true);
        proposal.approvals_weight = proposal.approvals_weight + weight;

        event::emit(ProposalApproved {
            proposal_id,
            approver: approver_addr,
            weight,
            total_weight: proposal.approvals_weight,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Execute a proposal (if quorum met)
    public entry fun execute_proposal(
        proposal_id: u64,
    ) acquires SignerListRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.proposals, proposal_id), error::not_found(E_PROPOSAL_NOT_FOUND));

        let proposal = table::borrow_mut(&mut registry.proposals, proposal_id);
        assert!(!proposal.executed, error::invalid_state(E_ALREADY_EXECUTED));

        let signer_list = table::borrow(&registry.signer_lists, proposal.signer_list_id);
        assert!(proposal.approvals_weight >= signer_list.quorum, error::permission_denied(E_QUORUM_NOT_MET));

        proposal.executed = true;

        event::emit(ProposalExecuted {
            proposal_id,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// View functions
    #[view]
    public fun get_signer_list(list_id: u64): (address, u64, u64) acquires SignerListRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.signer_lists, list_id), error::not_found(E_LIST_NOT_FOUND));

        let signer_list = table::borrow(&registry.signer_lists, list_id);
        (signer_list.owner, signer_list.quorum, signer_list.total_weight)
    }

    #[view]
    public fun get_proposal(proposal_id: u64): (u64, address, u64, bool) acquires SignerListRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.proposals, proposal_id), error::not_found(E_PROPOSAL_NOT_FOUND));

        let proposal = table::borrow(&registry.proposals, proposal_id);
        (proposal.signer_list_id, proposal.proposer, proposal.approvals_weight, proposal.executed)
    }

    #[view]
    public fun is_signer(list_id: u64, addr: address): bool acquires SignerListRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<SignerListRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<SignerListRegistry>(registry_addr);
        if (!table::contains(&registry.signer_lists, list_id)) {
            return false
        };

        let signer_list = table::borrow(&registry.signer_lists, list_id);
        simple_map::contains_key(&signer_list.signer_weights, &addr)
    }

    #[view]
    public fun get_signer_weight(list_id: u64, addr: address): u64 acquires SignerListRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<SignerListRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<SignerListRegistry>(registry_addr);
        assert!(table::contains(&registry.signer_lists, list_id), error::not_found(E_LIST_NOT_FOUND));

        let signer_list = table::borrow(&registry.signer_lists, list_id);
        assert!(simple_map::contains_key(&signer_list.signer_weights, &addr), error::not_found(E_SIGNER_NOT_FOUND));

        *simple_map::borrow(&signer_list.signer_weights, &addr)
    }
}
