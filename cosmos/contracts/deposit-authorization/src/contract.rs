use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    AuthResponse, AuthorizedListResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SettingsResponse,
};
use crate::state::{AuthSettings, AUTHORIZED, AUTH_SETTINGS, TOKEN_AUTH};

const CONTRACT_NAME: &str = "crates.io:deposit-authorization";
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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::EnableDepositAuth {} => execute_enable_deposit_auth(deps, info),
        ExecuteMsg::DisableDepositAuth {} => execute_disable_deposit_auth(deps, info),
        ExecuteMsg::UpdateSettings {
            whitelist_mode,
            require_auth_for_all_tokens,
        } => execute_update_settings(deps, info, whitelist_mode, require_auth_for_all_tokens),
        ExecuteMsg::AuthorizeDepositor { depositor } => {
            execute_authorize_depositor(deps, info, depositor)
        }
        ExecuteMsg::UnauthorizeDepositor { depositor } => {
            execute_unauthorize_depositor(deps, info, depositor)
        }
        ExecuteMsg::AuthorizeToken { depositor, token } => {
            execute_authorize_token(deps, info, depositor, token)
        }
        ExecuteMsg::UnauthorizeToken { depositor, token } => {
            execute_unauthorize_token(deps, info, depositor, token)
        }
    }
}

pub fn execute_enable_deposit_auth(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let settings = AuthSettings {
        enabled: true,
        whitelist_mode: true, // Default to whitelist mode
        require_auth_for_all_tokens: false,
    };

    AUTH_SETTINGS.save(deps.storage, &info.sender, &settings)?;

    Ok(Response::new()
        .add_attribute("method", "enable_deposit_auth")
        .add_attribute("account", info.sender))
}

pub fn execute_disable_deposit_auth(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    AUTH_SETTINGS.update(deps.storage, &info.sender, |maybe_settings| {
        let mut settings = maybe_settings.ok_or(ContractError::NotEnabled {})?;
        settings.enabled = false;
        Ok::<_, ContractError>(settings)
    })?;

    Ok(Response::new()
        .add_attribute("method", "disable_deposit_auth")
        .add_attribute("account", info.sender))
}

pub fn execute_update_settings(
    deps: DepsMut,
    info: MessageInfo,
    whitelist_mode: Option<bool>,
    require_auth_for_all_tokens: Option<bool>,
) -> Result<Response, ContractError> {
    AUTH_SETTINGS.update(deps.storage, &info.sender, |maybe_settings| {
        let mut settings = maybe_settings.ok_or(ContractError::NotEnabled {})?;

        if let Some(wl) = whitelist_mode {
            settings.whitelist_mode = wl;
        }
        if let Some(req) = require_auth_for_all_tokens {
            settings.require_auth_for_all_tokens = req;
        }

        Ok::<_, ContractError>(settings)
    })?;

    Ok(Response::new()
        .add_attribute("method", "update_settings")
        .add_attribute("account", info.sender))
}

pub fn execute_authorize_depositor(
    deps: DepsMut,
    info: MessageInfo,
    depositor: String,
) -> Result<Response, ContractError> {
    let depositor_addr = deps.api.addr_validate(&depositor)?;

    // Check if already authorized
    if AUTHORIZED
        .may_load(deps.storage, (&info.sender, &depositor_addr))?
        .unwrap_or(false)
    {
        return Err(ContractError::AlreadyAuthorized {});
    }

    AUTHORIZED.save(deps.storage, (&info.sender, &depositor_addr), &true)?;

    Ok(Response::new()
        .add_attribute("method", "authorize_depositor")
        .add_attribute("account", info.sender.to_string())
        .add_attribute("depositor", depositor))
}

pub fn execute_unauthorize_depositor(
    deps: DepsMut,
    info: MessageInfo,
    depositor: String,
) -> Result<Response, ContractError> {
    let depositor_addr = deps.api.addr_validate(&depositor)?;

    // Check if not authorized
    if !AUTHORIZED
        .may_load(deps.storage, (&info.sender, &depositor_addr))?
        .unwrap_or(false)
    {
        return Err(ContractError::AlreadyUnauthorized {});
    }

    AUTHORIZED.save(deps.storage, (&info.sender, &depositor_addr), &false)?;

    Ok(Response::new()
        .add_attribute("method", "unauthorize_depositor")
        .add_attribute("account", info.sender.to_string())
        .add_attribute("depositor", depositor))
}

pub fn execute_authorize_token(
    deps: DepsMut,
    info: MessageInfo,
    depositor: String,
    token: String,
) -> Result<Response, ContractError> {
    let depositor_addr = deps.api.addr_validate(&depositor)?;
    TOKEN_AUTH.save(
        deps.storage,
        (&info.sender, &depositor_addr, &token),
        &true,
    )?;

    Ok(Response::new()
        .add_attribute("method", "authorize_token")
        .add_attribute("account", info.sender.to_string())
        .add_attribute("depositor", depositor)
        .add_attribute("token", token))
}

pub fn execute_unauthorize_token(
    deps: DepsMut,
    info: MessageInfo,
    depositor: String,
    token: String,
) -> Result<Response, ContractError> {
    let depositor_addr = deps.api.addr_validate(&depositor)?;
    TOKEN_AUTH.save(
        deps.storage,
        (&info.sender, &depositor_addr, &token),
        &false,
    )?;

    Ok(Response::new()
        .add_attribute("method", "unauthorize_token")
        .add_attribute("account", info.sender.to_string())
        .add_attribute("depositor", depositor)
        .add_attribute("token", token))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetSettings { account } => to_json_binary(&query_settings(deps, account)?),
        QueryMsg::IsAuthorized {
            account,
            depositor,
            token,
        } => to_json_binary(&query_is_authorized(deps, account, depositor, token)?),
        QueryMsg::GetAuthorizedList { account } => {
            to_json_binary(&query_authorized_list(deps, account)?)
        }
    }
}

fn query_settings(deps: Deps, account: String) -> StdResult<SettingsResponse> {
    let account_addr = deps.api.addr_validate(&account)?;
    let settings = AUTH_SETTINGS
        .may_load(deps.storage, &account_addr)?
        .unwrap_or(AuthSettings {
            enabled: false,
            whitelist_mode: true,
            require_auth_for_all_tokens: false,
        });

    Ok(SettingsResponse {
        enabled: settings.enabled,
        whitelist_mode: settings.whitelist_mode,
        require_auth_for_all_tokens: settings.require_auth_for_all_tokens,
    })
}

fn query_is_authorized(
    deps: Deps,
    account: String,
    depositor: String,
    token: Option<String>,
) -> StdResult<AuthResponse> {
    let account_addr = deps.api.addr_validate(&account)?;
    let depositor_addr = deps.api.addr_validate(&depositor)?;

    let settings = AUTH_SETTINGS
        .may_load(deps.storage, &account_addr)?
        .unwrap_or(AuthSettings {
            enabled: false,
            whitelist_mode: true,
            require_auth_for_all_tokens: false,
        });

    if !settings.enabled {
        return Ok(AuthResponse {
            authorized: true,
            reason: "Deposit authorization not enabled".to_string(),
        });
    }

    // Check token-specific auth if provided
    if let Some(token) = token {
        let token_auth = TOKEN_AUTH
            .may_load(deps.storage, (&account_addr, &depositor_addr, &token))?
            .unwrap_or(false);

        if settings.require_auth_for_all_tokens {
            return Ok(AuthResponse {
                authorized: token_auth,
                reason: if token_auth {
                    "Token-specific authorization granted".to_string()
                } else {
                    "Token-specific authorization required".to_string()
                },
            });
        }
    }

    // Check general authorization
    let is_authorized = AUTHORIZED
        .may_load(deps.storage, (&account_addr, &depositor_addr))?
        .unwrap_or(false);

    let (authorized, reason) = if settings.whitelist_mode {
        // Whitelist mode: must be on the list
        (
            is_authorized,
            if is_authorized {
                "On whitelist".to_string()
            } else {
                "Not on whitelist".to_string()
            },
        )
    } else {
        // Blacklist mode: must NOT be on the list
        (
            !is_authorized,
            if is_authorized {
                "On blacklist".to_string()
            } else {
                "Not on blacklist".to_string()
            },
        )
    };

    Ok(AuthResponse { authorized, reason })
}

fn query_authorized_list(deps: Deps, account: String) -> StdResult<AuthorizedListResponse> {
    let account_addr = deps.api.addr_validate(&account)?;

    let depositors: Vec<Addr> = AUTHORIZED
        .prefix(&account_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| {
            let (depositor, is_auth) = item.ok()?;
            if is_auth {
                Some(depositor)
            } else {
                None
            }
        })
        .collect();

    Ok(AuthorizedListResponse { depositors })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::coins;

    #[test]
    fn enable_and_authorize() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Enable deposit auth
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::EnableDepositAuth {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Authorize depositor
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::AuthorizeDepositor {
            depositor: "bob".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.attributes.iter().any(|attr| attr.key == "depositor"));
    }
}
