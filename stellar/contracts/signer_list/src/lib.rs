#\![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, BytesN, Env, Vec, vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerEntry { pub signer: Address, pub weight: u32 }

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerList { pub owner: Address, pub signers: Vec<SignerEntry>, pub quorum: u32, pub created_at: u64, pub updated_at: u64 }

#[contracttype]
#[derive(Clone)]
pub enum DataKey { SignerList(Address), PendingTx(BytesN<32>), Admin }

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingTransaction { pub tx_hash: BytesN<32>, pub signers: Vec<Address>, pub total_weight: u32, pub executed: bool }

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error { NotFound = 1, Unauthorized = 2, InvalidWeight = 3, InvalidQuorum = 4, AlreadySigned = 5, InsufficientSignatures = 6, AlreadyExecuted = 7 }

#[contract]
pub struct SignerListContract;

#[contractimpl]
impl SignerListContract {
    pub fn initialize(env: Env, admin: Address) { admin.require_auth(); env.storage().instance().set(&DataKey::Admin, &admin); }

    pub fn create_signer_list(env: Env, signers: Vec<SignerEntry>, quorum: u32) -> Result<(), Error> {
        let owner = env.invoker(); owner.require_auth();
        if quorum == 0 { return Err(Error::InvalidQuorum); }
        let total_weight: u32 = signers.iter().map(|s| s.weight).sum();
        if quorum > total_weight { return Err(Error::InvalidQuorum); }
        let list = SignerList { owner: owner.clone(), signers, quorum, created_at: env.ledger().timestamp(), updated_at: env.ledger().timestamp() };
        env.storage().persistent().set(&DataKey::SignerList(owner.clone()), &list);
        env.storage().persistent().extend_ttl(&DataKey::SignerList(owner.clone()), 518400, 518400);
        env.events().publish((symbol_short\!("created"), owner), quorum);
        Ok(())
    }

    pub fn add_signer(env: Env, signer: Address, weight: u32) -> Result<(), Error> {
        let owner = env.invoker(); owner.require_auth();
        let mut list: SignerList = env.storage().persistent().get(&DataKey::SignerList(owner.clone())).ok_or(Error::NotFound)?;
        if weight == 0 { return Err(Error::InvalidWeight); }
        list.signers.push_back(SignerEntry { signer, weight });
        list.updated_at = env.ledger().timestamp();
        env.storage().persistent().set(&DataKey::SignerList(owner.clone()), &list);
        Ok(())
    }

    pub fn sign_transaction(env: Env, owner: Address, tx_hash: BytesN<32>) -> Result<bool, Error> {
        let signer = env.invoker(); signer.require_auth();
        let list: SignerList = env.storage().persistent().get(&DataKey::SignerList(owner.clone())).ok_or(Error::NotFound)?;
        let signer_weight = list.signers.iter().find(|entry| entry.signer == signer).map(|entry| entry.weight).ok_or(Error::Unauthorized)?;
        let mut pending: PendingTransaction = env.storage().persistent().get(&DataKey::PendingTx(tx_hash.clone())).unwrap_or(PendingTransaction { tx_hash: tx_hash.clone(), signers: vec\![&env], total_weight: 0, executed: false });
        if pending.executed { return Err(Error::AlreadyExecuted); }
        for existing_signer in pending.signers.iter() { if existing_signer == signer { return Err(Error::AlreadySigned); } }
        pending.signers.push_back(signer.clone());
        pending.total_weight += signer_weight;
        env.storage().persistent().set(&DataKey::PendingTx(tx_hash.clone()), &pending);
        let ready = pending.total_weight >= list.quorum;
        if ready { env.events().publish((symbol_short\!("ready"), tx_hash), pending.total_weight); }
        Ok(ready)
    }

    pub fn get_signer_list(env: Env, owner: Address) -> Option<SignerList> { env.storage().persistent().get(&DataKey::SignerList(owner)) }
}
