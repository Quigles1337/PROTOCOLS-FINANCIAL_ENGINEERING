use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    AccountResponse, CanDeleteResponse, ConfigResponse, DeletedResponse, ExecuteMsg,
    InstantiateMsg, QueryMsg,
};
use crate::state::{Account, Config, ACCOUNTS, CONFIG};

const CONTRACT_NAME: &str = "crates.io:account-delete";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_MIN_ACCOUNT_AGE: u64 = 86400; // 24 hours

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        min_account_age: msg.min_account_age.unwrap_or(DEFAULT_MIN_ACCOUNT_AGE),
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("min_account_age", config.min_account_age.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateAccount {} => execute_create_account(deps, env, info),
        ExecuteMsg::DeleteAccount { beneficiary } => {
            execute_delete_account(deps, env, info, beneficiary)
        }
    }
}

pub fn execute_create_account(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let account = Account {
        owner: info.sender.clone(),
        created_at: env.block.time.seconds(),
        is_deleted: false,
        deleted_at: None,
        beneficiary: None,
    };

    ACCOUNTS.save(deps.storage, &info.sender, &account)?;

    Ok(Response::new()
        .add_attribute("method", "create_account")
        .add_attribute("owner", info.sender))
}

pub fn execute_delete_account(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    beneficiary: String,
) -> Result<Response, ContractError> {
    let beneficiary_addr = deps.api.addr_validate(&beneficiary)?;
    let config = CONFIG.load(deps.storage)?;

    ACCOUNTS.update(deps.storage, &info.sender, |maybe_account| {
        let mut account = maybe_account.ok_or(ContractError::AccountNotFound {})?;

        if account.is_deleted {
            return Err(ContractError::AlreadyDeleted {});
        }

        // Check account age
        let account_age = env.block.time.seconds() - account.created_at;
        if account_age < config.min_account_age {
            return Err(ContractError::AccountTooNew {
                min_age: config.min_account_age,
            });
        }

        // In production: Check that account has no outstanding balance

        account.is_deleted = true;
        account.deleted_at = Some(env.block.time.seconds());
        account.beneficiary = Some(beneficiary_addr.clone());

        Ok::<_, ContractError>(account)
    })?;

    Ok(Response::new()
        .add_attribute("method", "delete_account")
        .add_attribute("owner", info.sender)
        .add_attribute("beneficiary", beneficiary))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetAccount { owner } => to_json_binary(&query_account(deps, owner)?),
        QueryMsg::IsDeleted { owner } => to_json_binary(&query_is_deleted(deps, owner)?),
        QueryMsg::CanDelete { owner } => to_json_binary(&query_can_delete(deps, env, owner)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        min_account_age: config.min_account_age,
    })
}

fn query_account(deps: Deps, owner: String) -> StdResult<AccountResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let account = ACCOUNTS.load(deps.storage, &owner_addr)?;

    Ok(AccountResponse {
        owner: account.owner,
        created_at: account.created_at,
        is_deleted: account.is_deleted,
        deleted_at: account.deleted_at,
        beneficiary: account.beneficiary,
    })
}

fn query_is_deleted(deps: Deps, owner: String) -> StdResult<DeletedResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let account = ACCOUNTS.load(deps.storage, &owner_addr)?;

    Ok(DeletedResponse {
        is_deleted: account.is_deleted,
    })
}

fn query_can_delete(deps: Deps, env: Env, owner: String) -> StdResult<CanDeleteResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let account = ACCOUNTS.load(deps.storage, &owner_addr)?;
    let config = CONFIG.load(deps.storage)?;

    if account.is_deleted {
        return Ok(CanDeleteResponse {
            can_delete: false,
            reason: "Account already deleted".to_string(),
        });
    }

    let account_age = env.block.time.seconds() - account.created_at;
    if account_age < config.min_account_age {
        return Ok(CanDeleteResponse {
            can_delete: false,
            reason: format!(
                "Account too new (age: {}s, required: {}s)",
                account_age, config.min_account_age
            ),
        });
    }

    Ok(CanDeleteResponse {
        can_delete: true,
        reason: "Account can be deleted".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::coins;

    #[test]
    fn create_and_delete_account() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            min_account_age: Some(100),
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Create account
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::CreateAccount {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Try to delete immediately (should fail)
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::DeleteAccount {
            beneficiary: "bob".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::AccountTooNew { .. }));

        // Wait and delete
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(200);

        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::DeleteAccount {
            beneficiary: "bob".to_string(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert!(res.attributes.iter().any(|attr| attr.key == "beneficiary"));
    }
}
