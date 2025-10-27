module xrpl_primitives::signer_list {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::table::{Self, Table};
    use sui::vec_map::{Self, VecMap};
    use sui::event;
    use std::string::String;

    // Errors
    const ERR_NOT_AUTHORIZED: u64 = 1;
    const ERR_INVALID_QUORUM: u64 = 2;
    const ERR_INVALID_WEIGHT: u64 = 3;
    const ERR_SIGNER_EXISTS: u64 = 4;
    const ERR_SIGNER_NOT_FOUND: u64 = 5;
    const ERR_NOT_ACTIVE: u64 = 6;
    const ERR_NOT_SIGNER: u64 = 7;
    const ERR_ALREADY_APPROVED: u64 = 8;
    const ERR_QUORUM_NOT_MET: u64 = 9;
    const ERR_PROPOSAL_NOT_PENDING: u64 = 10;

    // Status
    const STATUS_PENDING: u8 = 0;
    const STATUS_EXECUTED: u8 = 1;
    const STATUS_REJECTED: u8 = 2;

    // Structs
    public struct SignerInfo has store, copy, drop {
        signer: address,
        weight: u64,
    }

    public struct SignerList has key {
        id: UID,
        owner: address,
        signers: VecMap<address, u64>,
        quorum: u64,
        active: bool,
        created_at: u64,
    }

    public struct Proposal has store {
        list_id: address,
        proposer: address,
        description: String,
        approvals: vector<address>,
        approval_weight: u64,
        status: u8,
        created_at: u64,
    }

    public struct ProposalRegistry has key {
        id: UID,
        proposals: Table<u64, Proposal>,
        next_id: u64,
    }

    // Events
    public struct SignerListCreated has copy, drop {
        list_id: address,
        owner: address,
        quorum: u64,
    }

    public struct SignerAdded has copy, drop {
        list_id: address,
        signer: address,
        weight: u64,
    }

    public struct ProposalCreated has copy, drop {
        proposal_id: u64,
        list_id: address,
    }

    public struct ProposalExecuted has copy, drop {
        proposal_id: u64,
    }

    // Initialize proposal registry
    fun init(ctx: &mut TxContext) {
        let registry = ProposalRegistry {
            id: object::new(ctx),
            proposals: table::new(ctx),
            next_id: 0,
        };
        transfer::share_object(registry);
    }

    // Create signer list
    public entry fun create_signer_list(
        quorum: u64,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(quorum > 0 && quorum <= 10000, ERR_INVALID_QUORUM);

        let signer_list = SignerList {
            id: object::new(ctx),
            owner,
            signers: vec_map::empty(),
            quorum,
            active: true,
            created_at: tx_context::epoch(ctx),
        };

        let list_id = object::uid_to_address(&signer_list.id);

        event::emit(SignerListCreated {
            list_id,
            owner,
            quorum,
        });

        transfer::share_object(signer_list);
    }

    // Add signer
    public entry fun add_signer(
        signer_list: &mut SignerList,
        new_signer: address,
        weight: u64,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(signer_list.owner == owner, ERR_NOT_AUTHORIZED);
        assert!(signer_list.active, ERR_NOT_ACTIVE);
        assert!(weight > 0 && weight <= 10000, ERR_INVALID_WEIGHT);
        assert!(!vec_map::contains(&signer_list.signers, &new_signer), ERR_SIGNER_EXISTS);

        vec_map::insert(&mut signer_list.signers, new_signer, weight);

        event::emit(SignerAdded {
            list_id: object::uid_to_address(&signer_list.id),
            signer: new_signer,
            weight,
        });
    }

    // Remove signer
    public entry fun remove_signer(
        signer_list: &mut SignerList,
        signer: address,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(signer_list.owner == owner, ERR_NOT_AUTHORIZED);
        assert!(signer_list.active, ERR_NOT_ACTIVE);
        assert!(vec_map::contains(&signer_list.signers, &signer), ERR_SIGNER_NOT_FOUND);

        let (_, _) = vec_map::remove(&mut signer_list.signers, &signer);
    }

    // Create proposal
    public entry fun create_proposal(
        registry: &mut ProposalRegistry,
        signer_list: &SignerList,
        description: String,
        ctx: &mut TxContext
    ): u64 {
        let proposer = tx_context::sender(ctx);
        assert!(signer_list.active, ERR_NOT_ACTIVE);
        assert!(vec_map::contains(&signer_list.signers, &proposer), ERR_NOT_SIGNER);

        let proposal_id = registry.next_id;
        registry.next_id = registry.next_id + 1;

        let proposal = Proposal {
            list_id: object::uid_to_address(&signer_list.id),
            proposer,
            description,
            approvals: vector::empty(),
            approval_weight: 0,
            status: STATUS_PENDING,
            created_at: tx_context::epoch(ctx),
        };

        table::add(&mut registry.proposals, proposal_id, proposal);

        event::emit(ProposalCreated {
            proposal_id,
            list_id: object::uid_to_address(&signer_list.id),
        });

        proposal_id
    }

    // Approve proposal
    public entry fun approve_proposal(
        registry: &mut ProposalRegistry,
        signer_list: &SignerList,
        proposal_id: u64,
        ctx: &mut TxContext
    ) {
        let approver = tx_context::sender(ctx);
        assert!(table::contains(&registry.proposals, proposal_id), ERR_PROPOSAL_NOT_PENDING);

        let proposal = table::borrow_mut(&mut registry.proposals, proposal_id);
        assert!(proposal.status == STATUS_PENDING, ERR_PROPOSAL_NOT_PENDING);
        assert!(vec_map::contains(&signer_list.signers, &approver), ERR_NOT_SIGNER);
        assert!(!vector::contains(&proposal.approvals, &approver), ERR_ALREADY_APPROVED);

        let weight = *vec_map::get(&signer_list.signers, &approver);
        vector::push_back(&mut proposal.approvals, approver);
        proposal.approval_weight = proposal.approval_weight + weight;
    }

    // Execute proposal
    public entry fun execute_proposal(
        registry: &mut ProposalRegistry,
        signer_list: &SignerList,
        proposal_id: u64,
        ctx: &mut TxContext
    ) {
        let executor = tx_context::sender(ctx);
        assert!(table::contains(&registry.proposals, proposal_id), ERR_PROPOSAL_NOT_PENDING);

        let proposal = table::borrow_mut(&mut registry.proposals, proposal_id);
        assert!(proposal.status == STATUS_PENDING, ERR_PROPOSAL_NOT_PENDING);
        assert!(signer_list.active, ERR_NOT_ACTIVE);
        assert!(vec_map::contains(&signer_list.signers, &executor), ERR_NOT_SIGNER);
        assert!(proposal.approval_weight >= signer_list.quorum, ERR_QUORUM_NOT_MET);

        proposal.status = STATUS_EXECUTED;

        event::emit(ProposalExecuted {
            proposal_id,
        });
    }

    // View functions
    public fun has_quorum(
        registry: &ProposalRegistry,
        signer_list: &SignerList,
        proposal_id: u64
    ): bool {
        if (table::contains(&registry.proposals, proposal_id)) {
            let proposal = table::borrow(&registry.proposals, proposal_id);
            proposal.approval_weight >= signer_list.quorum
        } else {
            false
        }
    }
}
