use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    CheckResponse, CheckStatusResponse, ChecksResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use crate::state::{Check, CheckStatus, CHECKS, NEXT_CHECK_ID, RECIPIENT_CHECKS, SENDER_CHECKS};

const CONTRACT_NAME: &str = "crates.io:checks";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    NEXT_CHECK_ID.save(deps.storage, &1u64)?;

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
        ExecuteMsg::CreateCheck {
            recipient,
            denom,
            amount,
            expiry,
            memo,
        } => execute_create_check(deps, env, info, recipient, denom, amount, expiry, memo),
        ExecuteMsg::CashCheck { check_id } => execute_cash_check(deps, env, info, check_id),
        ExecuteMsg::CancelCheck { check_id } => execute_cancel_check(deps, info, check_id),
    }
}

pub fn execute_create_check(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
    denom: String,
    amount: Uint128,
    expiry: Option<u64>,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    let recipient_addr = if let Some(rec) = recipient {
        Some(deps.api.addr_validate(&rec)?)
    } else {
        None
    };

    let check_id = NEXT_CHECK_ID.load(deps.storage)?;
    NEXT_CHECK_ID.save(deps.storage, &(check_id + 1))?;

    let check = Check {
        id: check_id,
        sender: info.sender.clone(),
        recipient: recipient_addr.clone(),
        denom: denom.clone(),
        amount,
        expiry: expiry.unwrap_or(0),
        memo: memo.unwrap_or_default(),
        created_at: env.block.time.seconds(),
        status: CheckStatus::Active,
        cashed_by: None,
        cashed_at: None,
    };

    CHECKS.save(deps.storage, check_id, &check)?;
    SENDER_CHECKS.save(deps.storage, (&info.sender, check_id), &())?;

    if let Some(ref recipient) = recipient_addr {
        RECIPIENT_CHECKS.save(deps.storage, (recipient, check_id), &())?;
    }

    Ok(Response::new()
        .add_attribute("method", "create_check")
        .add_attribute("check_id", check_id.to_string())
        .add_attribute("sender", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("denom", denom))
}

pub fn execute_cash_check(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    check_id: u64,
) -> Result<Response, ContractError> {
    CHECKS.update(deps.storage, check_id, |maybe_check| {
        let mut check = maybe_check.ok_or(ContractError::CheckNotFound {})?;

        // Cannot cash own check
        if info.sender == check.sender {
            return Err(ContractError::CannotCashOwnCheck {});
        }

        if !matches!(check.status, CheckStatus::Active) {
            return Err(ContractError::AlreadyCashed {});
        }

        // Check expiry
        if check.expiry > 0 && env.block.time.seconds() >= check.expiry {
            return Err(ContractError::CheckExpired {});
        }

        // If recipient is specified, only they can cash it
        if let Some(ref recipient) = check.recipient {
            if &info.sender != recipient {
                return Err(ContractError::Unauthorized {});
            }
        }

        // In production: Transfer funds from sender to casher
        check.status = CheckStatus::Cashed;
        check.cashed_by = Some(info.sender.clone());
        check.cashed_at = Some(env.block.time.seconds());
        Ok(check)
    })?;

    Ok(Response::new()
        .add_attribute("method", "cash_check")
        .add_attribute("check_id", check_id.to_string())
        .add_attribute("cashed_by", info.sender))
}

pub fn execute_cancel_check(
    deps: DepsMut,
    info: MessageInfo,
    check_id: u64,
) -> Result<Response, ContractError> {
    CHECKS.update(deps.storage, check_id, |maybe_check| {
        let mut check = maybe_check.ok_or(ContractError::CheckNotFound {})?;

        if info.sender != check.sender {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(check.status, CheckStatus::Active) {
            return Err(ContractError::AlreadyCancelled {});
        }

        check.status = CheckStatus::Cancelled;
        Ok(check)
    })?;

    Ok(Response::new()
        .add_attribute("method", "cancel_check")
        .add_attribute("check_id", check_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCheck { check_id } => to_json_binary(&query_check(deps, check_id)?),
        QueryMsg::GetChecksBySender {
            sender,
            start_after,
            limit,
        } => to_json_binary(&query_checks_by_sender(deps, sender, start_after, limit)?),
        QueryMsg::GetChecksByRecipient {
            recipient,
            start_after,
            limit,
        } => to_json_binary(&query_checks_by_recipient(
            deps, recipient, start_after, limit,
        )?),
        QueryMsg::GetCheckStatus { check_id } => {
            to_json_binary(&query_check_status(deps, env, check_id)?)
        }
    }
}

fn query_check(deps: Deps, check_id: u64) -> StdResult<CheckResponse> {
    let check = CHECKS.load(deps.storage, check_id)?;
    Ok(check_to_response(check))
}

fn query_checks_by_sender(
    deps: Deps,
    sender: String,
    _start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ChecksResponse> {
    let sender_addr = deps.api.addr_validate(&sender)?;
    let limit = limit.unwrap_or(10) as usize;

    let checks: Vec<CheckResponse> = SENDER_CHECKS
        .prefix(&sender_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            let (check_id, _) = item.ok()?;
            let check = CHECKS.load(deps.storage, check_id).ok()?;
            Some(check_to_response(check))
        })
        .collect();

    Ok(ChecksResponse { checks })
}

fn query_checks_by_recipient(
    deps: Deps,
    recipient: String,
    _start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ChecksResponse> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    let limit = limit.unwrap_or(10) as usize;

    let checks: Vec<CheckResponse> = RECIPIENT_CHECKS
        .prefix(&recipient_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            let (check_id, _) = item.ok()?;
            let check = CHECKS.load(deps.storage, check_id).ok()?;
            Some(check_to_response(check))
        })
        .collect();

    Ok(ChecksResponse { checks })
}

fn query_check_status(deps: Deps, env: Env, check_id: u64) -> StdResult<CheckStatusResponse> {
    let check = CHECKS.load(deps.storage, check_id)?;

    let is_expired = check.expiry > 0 && env.block.time.seconds() >= check.expiry;
    let can_cash = matches!(check.status, CheckStatus::Active) && !is_expired;

    Ok(CheckStatusResponse {
        status: check.status,
        can_cash,
        is_expired,
    })
}

fn check_to_response(check: Check) -> CheckResponse {
    CheckResponse {
        id: check.id,
        sender: check.sender,
        recipient: check.recipient,
        denom: check.denom,
        amount: check.amount,
        expiry: check.expiry,
        memo: check.memo,
        created_at: check.created_at,
        status: check.status,
        cashed_by: check.cashed_by,
        cashed_at: check.cashed_at,
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
    fn create_and_cash_check() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Create check
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::CreateCheck {
            recipient: Some("bob".to_string()),
            denom: "uatom".to_string(),
            amount: Uint128::new(500),
            expiry: None,
            memo: Some("Payroll".to_string()),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 5);

        // Cash check
        let info = mock_info("bob", &[]);
        let msg = ExecuteMsg::CashCheck { check_id: 1 };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.attributes.iter().any(|attr| attr.key == "cashed_by"));
    }
}
