use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

#[cw_serde]
pub struct AuthSettings {
    /// Is deposit authorization enabled
    pub enabled: bool,
    /// Whitelist mode (true) or blacklist mode (false)
    pub whitelist_mode: bool,
    /// Require authorization for all tokens
    pub require_auth_for_all_tokens: bool,
}

/// Authorization settings per account
pub const AUTH_SETTINGS: Map<&Addr, AuthSettings> = Map::new("auth_settings");

/// Authorized depositors (whitelist or blacklist depending on mode)
/// Key: (account, depositor)
pub const AUTHORIZED: Map<(&Addr, &Addr), bool> = Map::new("authorized");

/// Token-specific authorizations
/// Key: (account, depositor, token)
pub const TOKEN_AUTH: Map<(&Addr, &Addr, &str), bool> = Map::new("token_auth");
