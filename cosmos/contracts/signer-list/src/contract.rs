use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use std::collections::HashSet;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, HasListResponse, InstantiateMsg, QueryMsg, QuorumResponse, SignerInfo,
    SignerInput, SignerListResponse,
};
use crate::state::{Signer, SignerList, SIGNER_LISTS};

const CONTRACT_NAME: &str = "crates.io:signer-list";
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
        ExecuteMsg::SetSignerList { quorum, signers } => {
            execute_set_signer_list(deps, env, info, quorum, signers)
        }
        ExecuteMsg::RemoveSignerList {} => execute_remove_signer_list(deps, info),
        ExecuteMsg::VerifySignatures { signers } => {
            execute_verify_signatures(deps, info, signers)
        }
    }
}

pub fn execute_set_signer_list(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    quorum: u64,
    signers_input: Vec<SignerInput>,
) -> Result<Response, ContractError> {
    if signers_input.is_empty() {
        return Err(ContractError::EmptySignerList {});
    }

    // Check for duplicates
    let addresses: HashSet<String> = signers_input.iter().map(|s| s.address.clone()).collect();
    if addresses.len() != signers_input.len() {
        return Err(ContractError::DuplicateSigner {});
    }

    // Validate and convert signers
    let mut signers = Vec::new();
    let mut total_weight = 0u64;

    for signer_input in signers_input {
        let addr = deps.api.addr_validate(&signer_input.address)?;
        total_weight += signer_input.weight;
        signers.push(Signer {
            address: addr,
            weight: signer_input.weight,
        });
    }

    // Validate quorum
    if quorum == 0 || quorum > total_weight {
        return Err(ContractError::InvalidQuorum {});
    }

    let signer_list = SignerList {
        owner: info.sender.clone(),
        quorum,
        signers,
        total_weight,
        created_at: env.block.time.seconds(),
        updated_at: env.block.time.seconds(),
    };

    SIGNER_LISTS.save(deps.storage, &info.sender, &signer_list)?;

    Ok(Response::new()
        .add_attribute("method", "set_signer_list")
        .add_attribute("owner", info.sender)
        .add_attribute("quorum", quorum.to_string())
        .add_attribute("total_weight", total_weight.to_string()))
}

pub fn execute_remove_signer_list(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    if !SIGNER_LISTS.has(deps.storage, &info.sender) {
        return Err(ContractError::SignerListNotFound {});
    }

    SIGNER_LISTS.remove(deps.storage, &info.sender);

    Ok(Response::new()
        .add_attribute("method", "remove_signer_list")
        .add_attribute("owner", info.sender))
}

pub fn execute_verify_signatures(
    deps: DepsMut,
    info: MessageInfo,
    signers: Vec<String>,
) -> Result<Response, ContractError> {
    let signer_list = SIGNER_LISTS.load(deps.storage, &info.sender)?;

    let mut total_weight = 0u64;

    for signer_str in &signers {
        let signer_addr = deps.api.addr_validate(signer_str)?;
        if let Some(signer) = signer_list.signers.iter().find(|s| s.address == signer_addr) {
            total_weight += signer.weight;
        }
    }

    if total_weight < signer_list.quorum {
        return Err(ContractError::InsufficientWeight {
            quorum: signer_list.quorum,
            weight: total_weight,
        });
    }

    Ok(Response::new()
        .add_attribute("method", "verify_signatures")
        .add_attribute("weight", total_weight.to_string())
        .add_attribute("quorum", signer_list.quorum.to_string())
        .add_attribute("verified", "true"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetSignerList { owner } => to_json_binary(&query_signer_list(deps, owner)?),
        QueryMsg::HasSignerList { owner } => to_json_binary(&query_has_signer_list(deps, owner)?),
        QueryMsg::CheckQuorum { owner, signers } => {
            to_json_binary(&query_check_quorum(deps, owner, signers)?)
        }
    }
}

fn query_signer_list(deps: Deps, owner: String) -> StdResult<SignerListResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let signer_list = SIGNER_LISTS.load(deps.storage, &owner_addr)?;

    let signers: Vec<SignerInfo> = signer_list
        .signers
        .iter()
        .map(|s| SignerInfo {
            address: s.address.clone(),
            weight: s.weight,
        })
        .collect();

    Ok(SignerListResponse {
        owner: signer_list.owner,
        quorum: signer_list.quorum,
        signers,
        total_weight: signer_list.total_weight,
        created_at: signer_list.created_at,
        updated_at: signer_list.updated_at,
    })
}

fn query_has_signer_list(deps: Deps, owner: String) -> StdResult<HasListResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let has_list = SIGNER_LISTS.has(deps.storage, &owner_addr);

    Ok(HasListResponse { has_list })
}

fn query_check_quorum(deps: Deps, owner: String, signers: Vec<String>) -> StdResult<QuorumResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let signer_list = SIGNER_LISTS.load(deps.storage, &owner_addr)?;

    let mut total_weight = 0u64;

    for signer_str in &signers {
        let signer_addr = deps.api.addr_validate(signer_str)?;
        if let Some(signer) = signer_list.signers.iter().find(|s| s.address == signer_addr) {
            total_weight += signer.weight;
        }
    }

    Ok(QuorumResponse {
        meets_quorum: total_weight >= signer_list.quorum,
        weight: total_weight,
        quorum: signer_list.quorum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::coins;

    #[test]
    fn set_and_verify_signer_list() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Set signer list
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::SetSignerList {
            quorum: 2,
            signers: vec![
                SignerInput {
                    address: "signer1".to_string(),
                    weight: 1,
                },
                SignerInput {
                    address: "signer2".to_string(),
                    weight: 2,
                },
            ],
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 4);

        // Verify with sufficient weight
        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::VerifySignatures {
            signers: vec!["signer2".to_string()],
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.attributes.iter().any(|attr| attr.key == "verified"));
    }
}
