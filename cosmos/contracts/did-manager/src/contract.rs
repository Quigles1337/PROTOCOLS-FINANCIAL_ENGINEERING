use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{DIDResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ResolveResponse};
use crate::state::{DIDDocument, DIDS, DID_REVERSE};

const CONTRACT_NAME: &str = "crates.io:did-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_DOCUMENT_SIZE: usize = 10_000; // 10KB

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
        ExecuteMsg::SetDID { did_uri, document } => {
            execute_set_did(deps, env, info, did_uri, document)
        }
        ExecuteMsg::UpdateDID { document } => execute_update_did(deps, env, info, document),
        ExecuteMsg::DeleteDID {} => execute_delete_did(deps, info),
    }
}

pub fn execute_set_did(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    did_uri: String,
    document: String,
) -> Result<Response, ContractError> {
    // Validate DID URI format
    if !did_uri.starts_with("did:") {
        return Err(ContractError::InvalidDIDURI {});
    }

    // Check document size
    if document.len() > MAX_DOCUMENT_SIZE {
        return Err(ContractError::DocumentTooLarge {});
    }

    // Check if DID already exists for this owner
    if DIDS.may_load(deps.storage, &info.sender)?.is_some() {
        return Err(ContractError::DIDExists {});
    }

    // Check if DID URI is already taken
    if DID_REVERSE.may_load(deps.storage, did_uri.clone())?.is_some() {
        return Err(ContractError::DIDExists {});
    }

    let did_doc = DIDDocument {
        owner: info.sender.clone(),
        did_uri: did_uri.clone(),
        document: document.clone(),
        created_at: env.block.time.seconds(),
        updated_at: env.block.time.seconds(),
    };

    DIDS.save(deps.storage, &info.sender, &did_doc)?;
    DID_REVERSE.save(deps.storage, did_uri.clone(), &info.sender)?;

    Ok(Response::new()
        .add_attribute("method", "set_did")
        .add_attribute("owner", info.sender)
        .add_attribute("did_uri", did_uri))
}

pub fn execute_update_did(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    document: String,
) -> Result<Response, ContractError> {
    // Check document size
    if document.len() > MAX_DOCUMENT_SIZE {
        return Err(ContractError::DocumentTooLarge {});
    }

    DIDS.update(deps.storage, &info.sender, |maybe_did| {
        let mut did = maybe_did.ok_or(ContractError::DIDNotFound {})?;
        did.document = document.clone();
        did.updated_at = env.block.time.seconds();
        Ok::<_, ContractError>(did)
    })?;

    Ok(Response::new()
        .add_attribute("method", "update_did")
        .add_attribute("owner", info.sender))
}

pub fn execute_delete_did(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let did = DIDS.may_load(deps.storage, &info.sender)?
        .ok_or(ContractError::DIDNotFound {})?;

    // Remove from both indexes
    DIDS.remove(deps.storage, &info.sender);
    DID_REVERSE.remove(deps.storage, did.did_uri.clone());

    Ok(Response::new()
        .add_attribute("method", "delete_did")
        .add_attribute("owner", info.sender)
        .add_attribute("did_uri", did.did_uri))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDID { owner } => to_json_binary(&query_did(deps, owner)?),
        QueryMsg::ResolveDID { did_uri } => to_json_binary(&query_resolve_did(deps, did_uri)?),
    }
}

fn query_did(deps: Deps, owner: String) -> StdResult<DIDResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let did = DIDS.load(deps.storage, &owner_addr)?;

    Ok(DIDResponse {
        owner: did.owner,
        did_uri: did.did_uri,
        document: did.document,
        created_at: did.created_at,
        updated_at: did.updated_at,
    })
}

fn query_resolve_did(deps: Deps, did_uri: String) -> StdResult<ResolveResponse> {
    let owner = DID_REVERSE.load(deps.storage, did_uri.clone())?;
    let did = DIDS.load(deps.storage, &owner)?;

    Ok(ResolveResponse {
        owner: did.owner,
        document: did.document,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::coins;

    #[test]
    fn set_and_resolve_did() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::SetDID {
            did_uri: "did:cosmos:alice123".to_string(),
            document: r#"{"@context":"https://w3id.org/did/v1"}"#.to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 3);

        // Resolve DID
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ResolveDID {
                did_uri: "did:cosmos:alice123".to_string(),
            },
        )
        .unwrap();
        let response: ResolveResponse = cosmwasm_std::from_json(&res).unwrap();
        assert_eq!(response.owner.as_str(), "alice");
    }
}
