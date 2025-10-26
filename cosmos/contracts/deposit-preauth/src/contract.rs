use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, PreauthResponse, PreauthsResponse, QueryMsg, ValidResponse,
};
use crate::state::{Preauth, PREAUTHS};

const CONTRACT_NAME: &str = "crates.io:deposit-preauth";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
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
        ExecuteMsg::PreauthorizeDeposit {
            authorized,
            denom,
            max_amount,
            expires_at,
            memo,
        } => execute_preauthorize_deposit(
            deps,
            env,
            info,
            authorized,
            denom,
            max_amount,
            expires_at,
            memo,
        ),
        ExecuteMsg::RevokePreauth {
            authorized,
            denom,
        } => execute_revoke_preauth(deps, info, authorized, denom),
        ExecuteMsg::UsePreauth {
            authorizer,
            denom,
            amount,
        } => execute_use_preauth(deps, env, info, authorizer, denom, amount),
    }
}

pub fn execute_preauthorize_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    authorized: String,
    denom: String,
    max_amount: Uint128,
    expires_at: Option<u64>,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    let authorized_addr = deps.api.addr_validate(&authorized)?;

    let preauth = Preauth {
        authorizer: info.sender.clone(),
        authorized: authorized_addr.clone(),
        denom: denom.clone(),
        max_amount,
        expires_at: expires_at.unwrap_or(0),
        is_used: false,
        is_revoked: false,
        created_at: env.block.time.seconds(),
        memo: memo.unwrap_or_default(),
    };

    PREAUTHS.save(
        deps.storage,
        (&info.sender, &authorized_addr, &denom),
        &preauth,
    )?;

    Ok(Response::new()
        .add_attribute("method", "preauthorize_deposit")
        .add_attribute("authorizer", info.sender)
        .add_attribute("authorized", authorized)
        .add_attribute("denom", denom)
        .add_attribute("max_amount", max_amount))
}

pub fn execute_revoke_preauth(
    deps: DepsMut,
    info: MessageInfo,
    authorized: String,
    denom: String,
) -> Result<Response, ContractError> {
    let authorized_addr = deps.api.addr_validate(&authorized)?;

    PREAUTHS.update(
        deps.storage,
        (&info.sender, &authorized_addr, &denom),
        |maybe_preauth| {
            let mut preauth = maybe_preauth.ok_or(ContractError::PreauthNotFound {})?;

            if preauth.is_revoked {
                return Err(ContractError::Revoked {});
            }

            preauth.is_revoked = true;
            Ok::<_, ContractError>(preauth)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "revoke_preauth")
        .add_attribute("authorizer", info.sender)
        .add_attribute("authorized", authorized)
        .add_attribute("denom", denom))
}

pub fn execute_use_preauth(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    authorizer: String,
    denom: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let authorizer_addr = deps.api.addr_validate(&authorizer)?;

    PREAUTHS.update(
        deps.storage,
        (&authorizer_addr, &info.sender, &denom),
        |maybe_preauth| {
            let mut preauth = maybe_preauth.ok_or(ContractError::PreauthNotFound {})?;

            if preauth.is_used {
                return Err(ContractError::AlreadyUsed {});
            }

            if preauth.is_revoked {
                return Err(ContractError::Revoked {});
            }

            if preauth.expires_at > 0 && env.block.time.seconds() >= preauth.expires_at {
                return Err(ContractError::Expired {});
            }

            if amount > preauth.max_amount {
                return Err(ContractError::ExceedsMax {});
            }

            preauth.is_used = true;
            Ok::<_, ContractError>(preauth)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "use_preauth")
        .add_attribute("authorizer", authorizer)
        .add_attribute("authorized", info.sender)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPreauth {
            authorizer,
            authorized,
            denom,
        } => to_json_binary(&query_preauth(deps, authorizer, authorized, denom)?),
        QueryMsg::GetPreauthsByAuthorizer { authorizer } => {
            to_json_binary(&query_preauths_by_authorizer(deps, authorizer)?)
        }
        QueryMsg::GetPreauthsByAuthorized { authorized } => {
            to_json_binary(&query_preauths_by_authorized(deps, authorized)?)
        }
        QueryMsg::IsValid {
            authorizer,
            authorized,
            denom,
            amount,
        } => to_json_binary(&query_is_valid(
            deps, env, authorizer, authorized, denom, amount,
        )?),
    }
}

fn query_preauth(
    deps: Deps,
    authorizer: String,
    authorized: String,
    denom: String,
) -> StdResult<PreauthResponse> {
    let authorizer_addr = deps.api.addr_validate(&authorizer)?;
    let authorized_addr = deps.api.addr_validate(&authorized)?;

    let preauth = PREAUTHS.load(deps.storage, (&authorizer_addr, &authorized_addr, &denom))?;
    Ok(preauth_to_response(preauth))
}

fn query_preauths_by_authorizer(
    deps: Deps,
    authorizer: String,
) -> StdResult<PreauthsResponse> {
    let authorizer_addr = deps.api.addr_validate(&authorizer)?;

    let preauths: Vec<PreauthResponse> = PREAUTHS
        .prefix(&authorizer_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| {
            let (_, preauth) = item.ok()?;
            Some(preauth_to_response(preauth))
        })
        .collect();

    Ok(PreauthsResponse { preauths })
}

fn query_preauths_by_authorized(
    deps: Deps,
    authorized: String,
) -> StdResult<PreauthsResponse> {
    let authorized_addr = deps.api.addr_validate(&authorized)?;

    let preauths: Vec<PreauthResponse> = PREAUTHS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| {
            let (_, preauth) = item.ok()?;
            if preauth.authorized == authorized_addr {
                Some(preauth_to_response(preauth))
            } else {
                None
            }
        })
        .collect();

    Ok(PreauthsResponse { preauths })
}

fn query_is_valid(
    deps: Deps,
    env: Env,
    authorizer: String,
    authorized: String,
    denom: String,
    amount: Uint128,
) -> StdResult<ValidResponse> {
    let authorizer_addr = deps.api.addr_validate(&authorizer)?;
    let authorized_addr = deps.api.addr_validate(&authorized)?;

    let preauth = PREAUTHS.load(deps.storage, (&authorizer_addr, &authorized_addr, &denom))?;

    if preauth.is_used {
        return Ok(ValidResponse {
            valid: false,
            reason: "Already used".to_string(),
        });
    }

    if preauth.is_revoked {
        return Ok(ValidResponse {
            valid: false,
            reason: "Revoked".to_string(),
        });
    }

    if preauth.expires_at > 0 && env.block.time.seconds() >= preauth.expires_at {
        return Ok(ValidResponse {
            valid: false,
            reason: "Expired".to_string(),
        });
    }

    if amount > preauth.max_amount {
        return Ok(ValidResponse {
            valid: false,
            reason: "Exceeds maximum".to_string(),
        });
    }

    Ok(ValidResponse {
        valid: true,
        reason: "Valid".to_string(),
    })
}

fn preauth_to_response(preauth: Preauth) -> PreauthResponse {
    PreauthResponse {
        authorizer: preauth.authorizer,
        authorized: preauth.authorized,
        denom: preauth.denom,
        max_amount: preauth.max_amount,
        expires_at: preauth.expires_at,
        is_used: preauth.is_used,
        is_revoked: preauth.is_revoked,
        created_at: preauth.created_at,
        memo: preauth.memo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::coins;

    #[test]
    fn preauthorize_and_use() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Preauthorize
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::PreauthorizeDeposit {
            authorized: "bob".to_string(),
            denom: "uatom".to_string(),
            max_amount: Uint128::new(1000),
            expires_at: None,
            memo: Some("Invoice payment".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Use preauth
        let info = mock_info("bob", &[]);
        let msg = ExecuteMsg::UsePreauth {
            authorizer: "alice".to_string(),
            denom: "uatom".to_string(),
            amount: Uint128::new(500),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.attributes.iter().any(|attr| attr.key == "amount"));
    }
}
