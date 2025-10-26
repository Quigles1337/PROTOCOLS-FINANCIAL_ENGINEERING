use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    BalanceResponse, ChannelResponse, ChannelsResponse, ConfigResponse, ExecuteMsg,
    InstantiateMsg, QueryMsg,
};
use crate::state::{
    Channel, ChannelStatus, Config, CHANNELS, CONFIG, NEXT_CHANNEL_ID, RECIPIENT_CHANNELS,
    SENDER_CHANNELS,
};

const CONTRACT_NAME: &str = "crates.io:payment-channels";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_MIN_DURATION: u64 = 3600; // 1 hour
const DEFAULT_MAX_DURATION: u64 = 31_536_000; // 1 year
const DEFAULT_DISPUTE_PERIOD: u64 = 86400; // 24 hours

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        min_duration: msg.min_duration.unwrap_or(DEFAULT_MIN_DURATION),
        max_duration: msg.max_duration.unwrap_or(DEFAULT_MAX_DURATION),
        dispute_period: msg.dispute_period.unwrap_or(DEFAULT_DISPUTE_PERIOD),
    };
    CONFIG.save(deps.storage, &config)?;

    NEXT_CHANNEL_ID.save(deps.storage, &1u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("min_duration", config.min_duration.to_string())
        .add_attribute("max_duration", config.max_duration.to_string())
        .add_attribute("dispute_period", config.dispute_period.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateChannel {
            recipient,
            denom,
            amount,
            duration,
        } => execute_create_channel(deps, env, info, recipient, denom, amount, duration),
        ExecuteMsg::FundChannel { channel_id, amount } => {
            execute_fund_channel(deps, info, channel_id, amount)
        }
        ExecuteMsg::ExtendChannel {
            channel_id,
            duration,
        } => execute_extend_channel(deps, env, info, channel_id, duration),
        ExecuteMsg::ClaimPayment {
            channel_id,
            amount,
            nonce,
            signature,
        } => execute_claim_payment(deps, env, info, channel_id, amount, nonce, signature),
        ExecuteMsg::CloseChannel {
            channel_id,
            final_amount,
        } => execute_close_channel(deps, env, info, channel_id, final_amount),
        ExecuteMsg::CloseChannelUnilateral { channel_id } => {
            execute_close_channel_unilateral(deps, env, info, channel_id)
        }
        ExecuteMsg::DisputeClaim { channel_id } => {
            execute_dispute_claim(deps, env, info, channel_id)
        }
        ExecuteMsg::ResolveDispute { channel_id } => {
            execute_resolve_dispute(deps, env, info, channel_id)
        }
    }
}

pub fn execute_create_channel(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    denom: String,
    amount: Uint128,
    duration: u64,
) -> Result<Response, ContractError> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    if info.sender == recipient_addr {
        return Err(ContractError::InvalidRecipient {});
    }

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    let config = CONFIG.load(deps.storage)?;
    if duration < config.min_duration || duration > config.max_duration {
        return Err(ContractError::InvalidDuration {});
    }

    let channel_id = NEXT_CHANNEL_ID.load(deps.storage)?;
    NEXT_CHANNEL_ID.save(deps.storage, &(channel_id + 1))?;

    let channel = Channel {
        id: channel_id,
        sender: info.sender.clone(),
        recipient: recipient_addr.clone(),
        denom: denom.clone(),
        balance: amount,
        claimed: Uint128::zero(),
        nonce: 0,
        created_at: env.block.time.seconds(),
        expires_at: env.block.time.seconds() + duration,
        status: ChannelStatus::Active,
        disputed_at: None,
        disputed_amount: None,
    };

    CHANNELS.save(deps.storage, channel_id, &channel)?;
    SENDER_CHANNELS.save(deps.storage, (&info.sender, channel_id), &())?;
    RECIPIENT_CHANNELS.save(deps.storage, (&recipient_addr, channel_id), &())?;

    Ok(Response::new()
        .add_attribute("method", "create_channel")
        .add_attribute("channel_id", channel_id.to_string())
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount)
        .add_attribute("duration", duration.to_string()))
}

pub fn execute_fund_channel(
    deps: DepsMut,
    info: MessageInfo,
    channel_id: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    CHANNELS.update(deps.storage, channel_id, |maybe_channel| {
        let mut channel = maybe_channel.ok_or(ContractError::ChannelNotFound {})?;

        if info.sender != channel.sender {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(channel.status, ChannelStatus::Active) {
            return Err(ContractError::ChannelClosed {});
        }

        channel.balance = channel.balance.checked_add(amount)?;
        Ok(channel)
    })?;

    Ok(Response::new()
        .add_attribute("method", "fund_channel")
        .add_attribute("channel_id", channel_id.to_string())
        .add_attribute("amount", amount))
}

pub fn execute_extend_channel(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u64,
    duration: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    CHANNELS.update(deps.storage, channel_id, |maybe_channel| {
        let mut channel = maybe_channel.ok_or(ContractError::ChannelNotFound {})?;

        if info.sender != channel.sender {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(channel.status, ChannelStatus::Active) {
            return Err(ContractError::ChannelClosed {});
        }

        let new_expiry = channel.expires_at + duration;
        let max_expiry = env.block.time.seconds() + config.max_duration;
        if new_expiry > max_expiry {
            return Err(ContractError::InvalidDuration {});
        }

        channel.expires_at = new_expiry;
        Ok(channel)
    })?;

    Ok(Response::new()
        .add_attribute("method", "extend_channel")
        .add_attribute("channel_id", channel_id.to_string())
        .add_attribute("duration", duration.to_string()))
}

pub fn execute_claim_payment(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u64,
    amount: Uint128,
    nonce: u64,
    _signature: Binary,
) -> Result<Response, ContractError> {
    CHANNELS.update(deps.storage, channel_id, |maybe_channel| {
        let mut channel = maybe_channel.ok_or(ContractError::ChannelNotFound {})?;

        if info.sender != channel.recipient {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(channel.status, ChannelStatus::Active) {
            return Err(ContractError::ChannelClosed {});
        }

        if nonce <= channel.nonce {
            return Err(ContractError::InvalidNonce {});
        }

        if amount > channel.balance {
            return Err(ContractError::InsufficientBalance {});
        }

        // TODO: Verify signature
        // In production, verify that signature is from channel.sender
        // signing message with (channel_id, amount, nonce)

        let claim_amount = amount.checked_sub(channel.claimed)?;
        channel.claimed = amount;
        channel.nonce = nonce;

        // In production, transfer claim_amount to recipient here
        let _ = claim_amount; // Suppress unused warning

        // Check if channel should auto-close (fully claimed or expired)
        if channel.claimed >= channel.balance || env.block.time.seconds() >= channel.expires_at {
            channel.status = ChannelStatus::Closed;
        }

        Ok(channel)
    })?;

    Ok(Response::new()
        .add_attribute("method", "claim_payment")
        .add_attribute("channel_id", channel_id.to_string())
        .add_attribute("amount", amount)
        .add_attribute("nonce", nonce.to_string()))
}

pub fn execute_close_channel(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    channel_id: u64,
    final_amount: Uint128,
) -> Result<Response, ContractError> {
    CHANNELS.update(deps.storage, channel_id, |maybe_channel| {
        let mut channel = maybe_channel.ok_or(ContractError::ChannelNotFound {})?;

        if info.sender != channel.sender && info.sender != channel.recipient {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(channel.status, ChannelStatus::Active) {
            return Err(ContractError::ChannelClosed {});
        }

        if final_amount > channel.balance {
            return Err(ContractError::InsufficientBalance {});
        }

        // In production:
        // - Transfer final_amount to recipient
        // - Return (balance - final_amount) to sender

        channel.status = ChannelStatus::Closed;
        channel.claimed = final_amount;
        Ok(channel)
    })?;

    Ok(Response::new()
        .add_attribute("method", "close_channel")
        .add_attribute("channel_id", channel_id.to_string())
        .add_attribute("final_amount", final_amount))
}

pub fn execute_close_channel_unilateral(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u64,
) -> Result<Response, ContractError> {
    CHANNELS.update(deps.storage, channel_id, |maybe_channel| {
        let mut channel = maybe_channel.ok_or(ContractError::ChannelNotFound {})?;

        if info.sender != channel.sender && info.sender != channel.recipient {
            return Err(ContractError::Unauthorized {});
        }

        if env.block.time.seconds() < channel.expires_at {
            return Err(ContractError::ChannelNotExpired {});
        }

        if matches!(channel.status, ChannelStatus::Disputed) {
            let config = CONFIG.load(deps.storage)?;
            if let Some(disputed_at) = channel.disputed_at {
                if env.block.time.seconds() < disputed_at + config.dispute_period {
                    return Err(ContractError::DisputePeriodActive {});
                }
            }
        }

        // In production:
        // - Transfer claimed amount to recipient
        // - Return remainder to sender

        channel.status = ChannelStatus::Closed;
        Ok(channel)
    })?;

    Ok(Response::new()
        .add_attribute("method", "close_channel_unilateral")
        .add_attribute("channel_id", channel_id.to_string()))
}

pub fn execute_dispute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u64,
) -> Result<Response, ContractError> {
    CHANNELS.update(deps.storage, channel_id, |maybe_channel| {
        let mut channel = maybe_channel.ok_or(ContractError::ChannelNotFound {})?;

        if info.sender != channel.sender {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(channel.status, ChannelStatus::Active) {
            return Err(ContractError::ChannelClosed {});
        }

        channel.status = ChannelStatus::Disputed;
        channel.disputed_at = Some(env.block.time.seconds());
        channel.disputed_amount = Some(channel.claimed);
        Ok(channel)
    })?;

    Ok(Response::new()
        .add_attribute("method", "dispute_claim")
        .add_attribute("channel_id", channel_id.to_string()))
}

pub fn execute_resolve_dispute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    CHANNELS.update(deps.storage, channel_id, |maybe_channel| {
        let mut channel = maybe_channel.ok_or(ContractError::ChannelNotFound {})?;

        if info.sender != channel.sender && info.sender != channel.recipient {
            return Err(ContractError::Unauthorized {});
        }

        if !matches!(channel.status, ChannelStatus::Disputed) {
            return Err(ContractError::NoDispute {});
        }

        if let Some(disputed_at) = channel.disputed_at {
            if env.block.time.seconds() < disputed_at + config.dispute_period {
                return Err(ContractError::DisputePeriodActive {});
            }
        }

        // Dispute period has passed, close channel
        channel.status = ChannelStatus::Closed;
        Ok(channel)
    })?;

    Ok(Response::new()
        .add_attribute("method", "resolve_dispute")
        .add_attribute("channel_id", channel_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetChannel { channel_id } => to_json_binary(&query_channel(deps, channel_id)?),
        QueryMsg::GetChannelsBySender {
            sender,
            start_after,
            limit,
        } => to_json_binary(&query_channels_by_sender(deps, sender, start_after, limit)?),
        QueryMsg::GetChannelsByRecipient {
            recipient,
            start_after,
            limit,
        } => to_json_binary(&query_channels_by_recipient(
            deps,
            recipient,
            start_after,
            limit,
        )?),
        QueryMsg::GetAvailableBalance { channel_id } => {
            to_json_binary(&query_available_balance(deps, channel_id)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        min_duration: config.min_duration,
        max_duration: config.max_duration,
        dispute_period: config.dispute_period,
    })
}

fn query_channel(deps: Deps, channel_id: u64) -> StdResult<ChannelResponse> {
    let channel = CHANNELS.load(deps.storage, channel_id)?;
    Ok(channel_to_response(channel))
}

fn query_channels_by_sender(
    deps: Deps,
    sender: String,
    _start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ChannelsResponse> {
    let sender_addr = deps.api.addr_validate(&sender)?;
    let limit = limit.unwrap_or(10) as usize;

    let channels: Vec<ChannelResponse> = SENDER_CHANNELS
        .prefix(&sender_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            let (channel_id, _) = item.ok()?;
            let channel = CHANNELS.load(deps.storage, channel_id).ok()?;
            Some(channel_to_response(channel))
        })
        .collect();

    Ok(ChannelsResponse { channels })
}

fn query_channels_by_recipient(
    deps: Deps,
    recipient: String,
    _start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ChannelsResponse> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    let limit = limit.unwrap_or(10) as usize;

    let channels: Vec<ChannelResponse> = RECIPIENT_CHANNELS
        .prefix(&recipient_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            let (channel_id, _) = item.ok()?;
            let channel = CHANNELS.load(deps.storage, channel_id).ok()?;
            Some(channel_to_response(channel))
        })
        .collect();

    Ok(ChannelsResponse { channels })
}

fn query_available_balance(deps: Deps, channel_id: u64) -> StdResult<BalanceResponse> {
    let channel = CHANNELS.load(deps.storage, channel_id)?;
    let available = channel.balance.checked_sub(channel.claimed).unwrap_or(Uint128::zero());

    Ok(BalanceResponse {
        total: channel.balance,
        claimed: channel.claimed,
        available,
    })
}

fn channel_to_response(channel: Channel) -> ChannelResponse {
    ChannelResponse {
        id: channel.id,
        sender: channel.sender,
        recipient: channel.recipient,
        denom: channel.denom,
        balance: channel.balance,
        claimed: channel.claimed,
        nonce: channel.nonce,
        created_at: channel.created_at,
        expires_at: channel.expires_at,
        status: channel.status,
        disputed_at: channel.disputed_at,
        disputed_amount: channel.disputed_amount,
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
        let msg = InstantiateMsg {
            min_duration: Some(1800),
            max_duration: Some(2_592_000),
            dispute_period: Some(43200),
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_json(&res).unwrap();
        assert_eq!(1800, value.min_duration);
    }

    #[test]
    fn create_channel() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            min_duration: None,
            max_duration: None,
            dispute_period: None,
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("alice", &[]);
        let msg = ExecuteMsg::CreateChannel {
            recipient: "bob".to_string(),
            denom: "uatom".to_string(),
            amount: Uint128::new(1000),
            duration: 86400,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 7);
    }
}
