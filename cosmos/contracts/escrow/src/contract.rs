use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;
use sha2::{Digest, Sha256};

use crate::error::ContractError;
use crate::msg::{
    EscrowResponse, EscrowsResponse, ExecuteMsg, InstantiateMsg, QueryMsg, UnlockStatusResponse,
};
use crate::state::{
    Escrow, EscrowStatus, ESCROWS, NEXT_ESCROW_ID, RECIPIENT_ESCROWS, SENDER_ESCROWS,
};

const CONTRACT_NAME: &str = "crates.io:escrow";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    NEXT_ESCROW_ID.save(deps.storage, &1u64)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateTimeLock {
            recipient,
            denom,
            amount,
            release_time,
            expiry_time,
            memo,
        } => execute_create_time_lock(
            deps,
            env,
            info,
            recipient,
            denom,
            amount,
            release_time,
            expiry_time,
            memo,
        ),
        ExecuteMsg::CreateHashLock {
            recipient,
            denom,
            amount,
            condition_hash,
            expiry_time,
            memo,
        } => execute_create_hash_lock(
            deps,
            env,
            info,
            recipient,
            denom,
            amount,
            condition_hash,
            expiry_time,
            memo,
        ),
        ExecuteMsg::CreateTimedHashLock {
            recipient,
            denom,
            amount,
            release_time,
            condition_hash,
            expiry_time,
            memo,
        } => execute_create_timed_hash_lock(
            deps,
            env,
            info,
            recipient,
            denom,
            amount,
            release_time,
            condition_hash,
            expiry_time,
            memo,
        ),
        ExecuteMsg::Release {
            escrow_id,
            preimage,
        } => execute_release(deps, env, info, escrow_id, preimage),
        ExecuteMsg::Cancel { escrow_id } => execute_cancel(deps, env, info, escrow_id),
    }
}

pub fn execute_create_time_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    denom: String,
    amount: Uint128,
    release_time: u64,
    expiry_time: u64,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    if release_time <= env.block.time.seconds() {
        return Err(ContractError::InvalidReleaseTime {});
    }

    let escrow_id = NEXT_ESCROW_ID.load(deps.storage)?;
    NEXT_ESCROW_ID.save(deps.storage, &(escrow_id + 1))?;

    let escrow = Escrow {
        id: escrow_id,
        sender: info.sender.clone(),
        recipient: recipient_addr.clone(),
        denom: denom.clone(),
        amount,
        release_time,
        condition_hash: String::new(),
        expiry_time,
        memo: memo.unwrap_or_default(),
        created_at: env.block.time.seconds(),
        status: EscrowStatus::Active,
    };

    ESCROWS.save(deps.storage, escrow_id, &escrow)?;
    SENDER_ESCROWS.save(deps.storage, (&info.sender, escrow_id), &())?;
    RECIPIENT_ESCROWS.save(deps.storage, (&recipient_addr, escrow_id), &())?;

    Ok(Response::new()
        .add_attribute("method", "create_time_lock")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount)
        .add_attribute("release_time", release_time.to_string()))
}

pub fn execute_create_hash_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    denom: String,
    amount: Uint128,
    condition_hash: String,
    expiry_time: u64,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    if condition_hash.is_empty() {
        return Err(ContractError::InvalidConditionHash {});
    }

    let escrow_id = NEXT_ESCROW_ID.load(deps.storage)?;
    NEXT_ESCROW_ID.save(deps.storage, &(escrow_id + 1))?;

    let escrow = Escrow {
        id: escrow_id,
        sender: info.sender.clone(),
        recipient: recipient_addr.clone(),
        denom: denom.clone(),
        amount,
        release_time: 0,
        condition_hash: condition_hash.clone(),
        expiry_time,
        memo: memo.unwrap_or_default(),
        created_at: env.block.time.seconds(),
        status: EscrowStatus::Active,
    };

    ESCROWS.save(deps.storage, escrow_id, &escrow)?;
    SENDER_ESCROWS.save(deps.storage, (&info.sender, escrow_id), &())?;
    RECIPIENT_ESCROWS.save(deps.storage, (&recipient_addr, escrow_id), &())?;

    Ok(Response::new()
        .add_attribute("method", "create_hash_lock")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount)
        .add_attribute("condition_hash", condition_hash))
}

pub fn execute_create_timed_hash_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    denom: String,
    amount: Uint128,
    release_time: u64,
    condition_hash: String,
    expiry_time: u64,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    if release_time <= env.block.time.seconds() {
        return Err(ContractError::InvalidReleaseTime {});
    }

    if condition_hash.is_empty() {
        return Err(ContractError::InvalidConditionHash {});
    }

    let escrow_id = NEXT_ESCROW_ID.load(deps.storage)?;
    NEXT_ESCROW_ID.save(deps.storage, &(escrow_id + 1))?;

    let escrow = Escrow {
        id: escrow_id,
        sender: info.sender.clone(),
        recipient: recipient_addr.clone(),
        denom: denom.clone(),
        amount,
        release_time,
        condition_hash: condition_hash.clone(),
        expiry_time,
        memo: memo.unwrap_or_default(),
        created_at: env.block.time.seconds(),
        status: EscrowStatus::Active,
    };

    ESCROWS.save(deps.storage, escrow_id, &escrow)?;
    SENDER_ESCROWS.save(deps.storage, (&info.sender, escrow_id), &())?;
    RECIPIENT_ESCROWS.save(deps.storage, (&recipient_addr, escrow_id), &())?;

    Ok(Response::new()
        .add_attribute("method", "create_timed_hash_lock")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount)
        .add_attribute("release_time", release_time.to_string())
        .add_attribute("condition_hash", condition_hash))
}

pub fn execute_release(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    escrow_id: u64,
    preimage: Option<String>,
) -> Result<Response, ContractError> {
    ESCROWS.update(deps.storage, escrow_id, |maybe_escrow| {
        let mut escrow = maybe_escrow.ok_or(ContractError::EscrowNotFound {})?;

        if info.sender != escrow.recipient {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(escrow.status, EscrowStatus::Active) {
            return Err(ContractError::AlreadyReleased {});
        }

        // Check expiry
        if escrow.expiry_time > 0 && env.block.time.seconds() >= escrow.expiry_time {
            return Err(ContractError::Expired {});
        }

        // Check time lock
        if escrow.release_time > 0 && env.block.time.seconds() < escrow.release_time {
            return Err(ContractError::NotUnlocked {});
        }

        // Check hash lock
        if !escrow.condition_hash.is_empty() {
            let preimage = preimage.ok_or(ContractError::InvalidPreimage {})?;
            let mut hasher = Sha256::new();
            hasher.update(preimage.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            if hash != escrow.condition_hash {
                return Err(ContractError::InvalidPreimage {});
            }
        }

        // In production: Transfer funds to recipient
        escrow.status = EscrowStatus::Released;
        Ok(escrow)
    })?;

    Ok(Response::new()
        .add_attribute("method", "release")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("recipient", info.sender))
}

pub fn execute_cancel(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    escrow_id: u64,
) -> Result<Response, ContractError> {
    ESCROWS.update(deps.storage, escrow_id, |maybe_escrow| {
        let mut escrow = maybe_escrow.ok_or(ContractError::EscrowNotFound {})?;

        if info.sender != escrow.sender {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(escrow.status, EscrowStatus::Active) {
            return Err(ContractError::AlreadyCancelled {});
        }

        // Can only cancel if expired
        if env.block.time.seconds() < escrow.expiry_time {
            return Err(ContractError::CannotCancel {});
        }

        // In production: Return funds to sender
        escrow.status = EscrowStatus::Cancelled;
        Ok(escrow)
    })?;

    Ok(Response::new()
        .add_attribute("method", "cancel")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("sender", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetEscrow { escrow_id } => to_json_binary(&query_escrow(deps, escrow_id)?),
        QueryMsg::GetEscrowsBySender {
            sender,
            start_after,
            limit,
        } => to_json_binary(&query_escrows_by_sender(deps, sender, start_after, limit)?),
        QueryMsg::GetEscrowsByRecipient {
            recipient,
            start_after,
            limit,
        } => to_json_binary(&query_escrows_by_recipient(
            deps, recipient, start_after, limit,
        )?),
        QueryMsg::IsUnlocked { escrow_id } => {
            to_json_binary(&query_is_unlocked(deps, env, escrow_id)?)
        }
    }
}

fn query_escrow(deps: Deps, escrow_id: u64) -> StdResult<EscrowResponse> {
    let escrow = ESCROWS.load(deps.storage, escrow_id)?;
    Ok(escrow_to_response(escrow))
}

fn query_escrows_by_sender(
    deps: Deps,
    sender: String,
    _start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<EscrowsResponse> {
    let sender_addr = deps.api.addr_validate(&sender)?;
    let limit = limit.unwrap_or(10) as usize;

    let escrows: Vec<EscrowResponse> = SENDER_ESCROWS
        .prefix(&sender_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            let (escrow_id, _) = item.ok()?;
            let escrow = ESCROWS.load(deps.storage, escrow_id).ok()?;
            Some(escrow_to_response(escrow))
        })
        .collect();

    Ok(EscrowsResponse { escrows })
}

fn query_escrows_by_recipient(
    deps: Deps,
    recipient: String,
    _start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<EscrowsResponse> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    let limit = limit.unwrap_or(10) as usize;

    let escrows: Vec<EscrowResponse> = RECIPIENT_ESCROWS
        .prefix(&recipient_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            let (escrow_id, _) = item.ok()?;
            let escrow = ESCROWS.load(deps.storage, escrow_id).ok()?;
            Some(escrow_to_response(escrow))
        })
        .collect();

    Ok(EscrowsResponse { escrows })
}

fn query_is_unlocked(deps: Deps, env: Env, escrow_id: u64) -> StdResult<UnlockStatusResponse> {
    let escrow = ESCROWS.load(deps.storage, escrow_id)?;

    let time_locked = escrow.release_time > env.block.time.seconds();
    let hash_locked = !escrow.condition_hash.is_empty();
    let unlocked = !time_locked && !hash_locked;

    let time_remaining = if time_locked {
        Some(escrow.release_time - env.block.time.seconds())
    } else {
        None
    };

    Ok(UnlockStatusResponse {
        unlocked,
        time_locked,
        hash_locked,
        time_remaining,
    })
}

fn escrow_to_response(escrow: Escrow) -> EscrowResponse {
    EscrowResponse {
        id: escrow.id,
        sender: escrow.sender,
        recipient: escrow.recipient,
        denom: escrow.denom,
        amount: escrow.amount,
        release_time: escrow.release_time,
        condition_hash: escrow.condition_hash,
        expiry_time: escrow.expiry_time,
        memo: escrow.memo,
        created_at: escrow.created_at,
        status: escrow.status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_json};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn create_time_lock() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(100);

        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::CreateTimeLock {
            recipient: "bob".to_string(),
            denom: "uatom".to_string(),
            amount: Uint128::new(1000),
            release_time: env.block.time.seconds() + 1000,
            expiry_time: env.block.time.seconds() + 2000,
            memo: Some("Test escrow".to_string()),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes.len(), 6);
    }
}
