use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, CreditResponse, ExecuteMsg, InstantiateMsg, PathResponse, QueryMsg,
    TrustLineResponse, TrustLinesResponse,
};
use crate::state::{trust_line_key, Config, TrustLine, CONFIG, TRUST_LINES};

const CONTRACT_NAME: &str = "crates.io:trust-lines";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_MAX_PATH_LENGTH: usize = 6;
const DEFAULT_MIN_QUALITY: u64 = 1_000_000_000; // 1.0 scaled by 1e9

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        max_path_length: msg.max_path_length.unwrap_or(DEFAULT_MAX_PATH_LENGTH),
        min_quality: msg.min_quality.unwrap_or(DEFAULT_MIN_QUALITY),
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("max_path_length", config.max_path_length.to_string())
        .add_attribute("min_quality", config.min_quality.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateTrustLine {
            counterparty,
            denom,
            limit,
            allow_rippling,
            quality_in,
            quality_out,
        } => execute_create_trust_line(
            deps,
            env,
            info,
            counterparty,
            denom,
            limit,
            allow_rippling,
            quality_in,
            quality_out,
        ),
        ExecuteMsg::UpdateTrustLine {
            counterparty,
            denom,
            limit,
            allow_rippling,
            quality_in,
            quality_out,
        } => execute_update_trust_line(
            deps,
            env,
            info,
            counterparty,
            denom,
            limit,
            allow_rippling,
            quality_in,
            quality_out,
        ),
        ExecuteMsg::CloseTrustLine {
            counterparty,
            denom,
        } => execute_close_trust_line(deps, info, counterparty, denom),
        ExecuteMsg::SendPayment {
            recipient,
            denom,
            amount,
        } => execute_send_payment(deps, env, info, recipient, denom, amount),
        ExecuteMsg::SendPaymentThroughPath {
            recipient,
            denom,
            amount,
            path,
            max_quality_loss,
        } => execute_send_payment_through_path(
            deps,
            env,
            info,
            recipient,
            denom,
            amount,
            path,
            max_quality_loss,
        ),
    }
}

pub fn execute_create_trust_line(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    counterparty: String,
    denom: String,
    limit: Uint128,
    allow_rippling: Option<bool>,
    quality_in: Option<u64>,
    quality_out: Option<u64>,
) -> Result<Response, ContractError> {
    let counterparty_addr = deps.api.addr_validate(&counterparty)?;

    // Cannot create trust line with self
    if info.sender == counterparty_addr {
        return Err(ContractError::SelfTrustLine {});
    }

    // Limit must be > 0
    if limit.is_zero() {
        return Err(ContractError::InvalidLimit {});
    }

    let key = trust_line_key(&info.sender, &counterparty_addr, &denom);

    // Check if trust line already exists
    if TRUST_LINES.may_load(deps.storage, key.clone())?.is_some() {
        return Err(ContractError::TrustLineExists {});
    }

    let (account1, account2, limit1, limit2) = if info.sender < counterparty_addr {
        (info.sender.clone(), counterparty_addr.clone(), limit, Uint128::zero())
    } else {
        (counterparty_addr.clone(), info.sender.clone(), Uint128::zero(), limit)
    };

    let trust_line = TrustLine {
        account1,
        account2,
        denom: denom.clone(),
        limit1,
        limit2,
        balance: 0,
        allow_rippling: allow_rippling.unwrap_or(true),
        quality_in: quality_in.unwrap_or(DEFAULT_MIN_QUALITY),
        quality_out: quality_out.unwrap_or(DEFAULT_MIN_QUALITY),
        created_at: env.block.time.seconds(),
        updated_at: env.block.time.seconds(),
    };

    TRUST_LINES.save(deps.storage, key, &trust_line)?;

    Ok(Response::new()
        .add_attribute("method", "create_trust_line")
        .add_attribute("creator", info.sender)
        .add_attribute("counterparty", counterparty)
        .add_attribute("denom", denom)
        .add_attribute("limit", limit))
}

pub fn execute_update_trust_line(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    counterparty: String,
    denom: String,
    limit: Option<Uint128>,
    allow_rippling: Option<bool>,
    quality_in: Option<u64>,
    quality_out: Option<u64>,
) -> Result<Response, ContractError> {
    let counterparty_addr = deps.api.addr_validate(&counterparty)?;
    let key = trust_line_key(&info.sender, &counterparty_addr, &denom);

    TRUST_LINES.update(deps.storage, key, |maybe_tl| -> Result<_, ContractError> {
        let mut tl = maybe_tl.ok_or(ContractError::TrustLineNotFound {})?;

        // Update the limit for the caller's side
        if let Some(new_limit) = limit {
            if info.sender == tl.account1 {
                tl.limit1 = new_limit;
            } else {
                tl.limit2 = new_limit;
            }
        }

        // Only update rippling/quality if caller is account1 (by convention)
        if info.sender == tl.account1 {
            if let Some(rippling) = allow_rippling {
                tl.allow_rippling = rippling;
            }
            if let Some(qi) = quality_in {
                tl.quality_in = qi;
            }
            if let Some(qo) = quality_out {
                tl.quality_out = qo;
            }
        }

        tl.updated_at = env.block.time.seconds();
        Ok(tl)
    })?;

    Ok(Response::new()
        .add_attribute("method", "update_trust_line")
        .add_attribute("updater", info.sender)
        .add_attribute("counterparty", counterparty)
        .add_attribute("denom", denom))
}

pub fn execute_close_trust_line(
    deps: DepsMut,
    info: MessageInfo,
    counterparty: String,
    denom: String,
) -> Result<Response, ContractError> {
    let counterparty_addr = deps.api.addr_validate(&counterparty)?;
    let key = trust_line_key(&info.sender, &counterparty_addr, &denom);

    let tl = TRUST_LINES.may_load(deps.storage, key.clone())?
        .ok_or(ContractError::TrustLineNotFound {})?;

    // Can only close if balance is zero
    if tl.balance != 0 {
        return Err(ContractError::InsufficientCredit {});
    }

    TRUST_LINES.remove(deps.storage, key);

    Ok(Response::new()
        .add_attribute("method", "close_trust_line")
        .add_attribute("closer", info.sender)
        .add_attribute("counterparty", counterparty)
        .add_attribute("denom", denom))
}

pub fn execute_send_payment(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    recipient: String,
    denom: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    let recipient_addr = deps.api.addr_validate(&recipient)?;
    let sender = _info.sender;
    let key = trust_line_key(&sender, &recipient_addr, &denom);

    TRUST_LINES.update(deps.storage, key, |maybe_tl| -> Result<_, ContractError> {
        let mut tl = maybe_tl.ok_or(ContractError::TrustLineNotFound {})?;

        let amount_i128 = amount.u128() as i128;

        // Determine direction and check limits
        if sender == tl.account1 {
            // Payment from account1 to account2: decrease balance (account2 owes less)
            let new_balance = tl.balance - amount_i128;
            let max_negative = -(tl.limit1.u128() as i128);
            if new_balance < max_negative {
                return Err(ContractError::InsufficientCredit {});
            }
            tl.balance = new_balance;
        } else {
            // Payment from account2 to account1: increase balance (account2 owes more)
            let new_balance = tl.balance + amount_i128;
            let max_positive = tl.limit2.u128() as i128;
            if new_balance > max_positive {
                return Err(ContractError::InsufficientCredit {});
            }
            tl.balance = new_balance;
        }

        tl.updated_at = env.block.time.seconds();
        Ok(tl)
    })?;

    Ok(Response::new()
        .add_attribute("method", "send_payment")
        .add_attribute("sender", sender)
        .add_attribute("recipient", recipient)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount))
}

pub fn execute_send_payment_through_path(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    denom: String,
    amount: Uint128,
    path: Vec<String>,
    _max_quality_loss: Option<u64>,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    if path.is_empty() {
        return Err(ContractError::EmptyPath {});
    }

    let config = CONFIG.load(deps.storage)?;
    if path.len() > config.max_path_length {
        return Err(ContractError::PathTooLong {
            max: config.max_path_length,
        });
    }

    // Validate all addresses in path
    let mut validated_path: Vec<Addr> = vec![info.sender.clone()];
    for addr in &path {
        validated_path.push(deps.api.addr_validate(addr)?);
    }
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    validated_path.push(recipient_addr.clone());

    // Process payment through each hop
    let mut current_amount = amount;
    for i in 0..validated_path.len() - 1 {
        let from = &validated_path[i];
        let to = &validated_path[i + 1];
        let key = trust_line_key(from, to, &denom);

        TRUST_LINES.update(deps.storage, key.clone(), |maybe_tl| -> Result<_, ContractError> {
            let mut tl = maybe_tl.ok_or(ContractError::TrustLineNotFound {})?;

            // Check rippling enabled (except for first and last hop)
            if i > 0 && i < validated_path.len() - 2 && !tl.allow_rippling {
                return Err(ContractError::RipplingDisabled {});
            }

            let amount_i128 = current_amount.u128() as i128;

            // Update balance based on direction
            if from == &tl.account1 {
                let new_balance = tl.balance - amount_i128;
                let max_negative = -(tl.limit1.u128() as i128);
                if new_balance < max_negative {
                    return Err(ContractError::InsufficientCredit {});
                }
                tl.balance = new_balance;
            } else {
                let new_balance = tl.balance + amount_i128;
                let max_positive = tl.limit2.u128() as i128;
                if new_balance > max_positive {
                    return Err(ContractError::InsufficientCredit {});
                }
                tl.balance = new_balance;
            }

            tl.updated_at = env.block.time.seconds();
            Ok(tl)
        })?;
    }

    Ok(Response::new()
        .add_attribute("method", "send_payment_through_path")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount)
        .add_attribute("hops", path.len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetTrustLine {
            account1,
            account2,
            denom,
        } => to_json_binary(&query_trust_line(deps, account1, account2, denom)?),
        QueryMsg::GetTrustLines {
            account,
            start_after,
            limit,
        } => to_json_binary(&query_trust_lines(deps, account, start_after, limit)?),
        QueryMsg::GetAvailableCredit { from, to, denom } => {
            to_json_binary(&query_available_credit(deps, from, to, denom)?)
        }
        QueryMsg::FindPath {
            from,
            to,
            denom,
            amount,
        } => to_json_binary(&query_find_path(deps, from, to, denom, amount)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        max_path_length: config.max_path_length,
        min_quality: config.min_quality,
    })
}

fn query_trust_line(
    deps: Deps,
    account1: String,
    account2: String,
    denom: String,
) -> StdResult<TrustLineResponse> {
    let addr1 = deps.api.addr_validate(&account1)?;
    let addr2 = deps.api.addr_validate(&account2)?;
    let key = trust_line_key(&addr1, &addr2, &denom);

    let tl = TRUST_LINES.load(deps.storage, key)?;

    Ok(TrustLineResponse {
        account1: tl.account1,
        account2: tl.account2,
        denom: tl.denom,
        limit1: tl.limit1,
        limit2: tl.limit2,
        balance: tl.balance,
        allow_rippling: tl.allow_rippling,
        quality_in: tl.quality_in,
        quality_out: tl.quality_out,
        created_at: tl.created_at,
        updated_at: tl.updated_at,
    })
}

fn query_trust_lines(
    deps: Deps,
    account: String,
    _start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TrustLinesResponse> {
    let addr = deps.api.addr_validate(&account)?;
    let limit = limit.unwrap_or(10) as usize;

    let trust_lines: Vec<TrustLineResponse> = TRUST_LINES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            let (_, tl) = item.ok()?;
            if tl.account1 == addr || tl.account2 == addr {
                Some(TrustLineResponse {
                    account1: tl.account1,
                    account2: tl.account2,
                    denom: tl.denom,
                    limit1: tl.limit1,
                    limit2: tl.limit2,
                    balance: tl.balance,
                    allow_rippling: tl.allow_rippling,
                    quality_in: tl.quality_in,
                    quality_out: tl.quality_out,
                    created_at: tl.created_at,
                    updated_at: tl.updated_at,
                })
            } else {
                None
            }
        })
        .take(limit)
        .collect();

    Ok(TrustLinesResponse { trust_lines })
}

fn query_available_credit(
    deps: Deps,
    from: String,
    to: String,
    denom: String,
) -> StdResult<CreditResponse> {
    let from_addr = deps.api.addr_validate(&from)?;
    let to_addr = deps.api.addr_validate(&to)?;
    let key = trust_line_key(&from_addr, &to_addr, &denom);

    let tl = TRUST_LINES.load(deps.storage, key)?;

    let (limit, used) = if from_addr == tl.account1 {
        // From account1 to account2
        let limit = tl.limit1;
        let used_i128 = -tl.balance;
        let used = if used_i128 > 0 {
            Uint128::new(used_i128 as u128)
        } else {
            Uint128::zero()
        };
        (limit, used)
    } else {
        // From account2 to account1
        let limit = tl.limit2;
        let used_i128 = tl.balance;
        let used = if used_i128 > 0 {
            Uint128::new(used_i128 as u128)
        } else {
            Uint128::zero()
        };
        (limit, used)
    };

    let available = limit.checked_sub(used).unwrap_or(Uint128::zero());

    Ok(CreditResponse {
        available,
        limit,
        used,
    })
}

fn query_find_path(
    _deps: Deps,
    _from: String,
    _to: String,
    _denom: String,
    _amount: Uint128,
) -> StdResult<PathResponse> {
    // Simplified path finding - would need graph traversal in production
    Ok(PathResponse {
        path: vec![],
        quality: DEFAULT_MIN_QUALITY,
        found: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_json};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            max_path_length: Some(5),
            min_quality: Some(950_000_000),
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Query config
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_json(&res).unwrap();
        assert_eq!(5, value.max_path_length);
        assert_eq!(950_000_000, value.min_quality);
    }

    #[test]
    fn create_trust_line() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            max_path_length: None,
            min_quality: None,
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Create trust line
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::CreateTrustLine {
            counterparty: "bob".to_string(),
            denom: "uatom".to_string(),
            limit: Uint128::new(1000),
            allow_rippling: Some(true),
            quality_in: None,
            quality_out: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 5);
    }
}
