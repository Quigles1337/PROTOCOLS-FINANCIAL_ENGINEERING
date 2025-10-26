// Shared types and utilities for XRPL Financial Primitives on CosmWasm

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct TokenInfo {
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub enum Status {
    Active,
    Inactive,
    Completed,
    Cancelled,
}

// Common helper functions

/// Calculate percentage with precision
pub fn calculate_percentage(amount: Uint128, percentage: u64) -> Uint128 {
    amount.multiply_ratio(percentage, 100u128)
}

/// Check if timestamp has expired
pub fn is_expired(current_time: u64, expiry: u64) -> bool {
    expiry > 0 && current_time >= expiry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentage() {
        let amount = Uint128::new(1000);
        let result = calculate_percentage(amount, 10);
        assert_eq!(result, Uint128::new(100));
    }

    #[test]
    fn test_is_expired() {
        assert!(is_expired(100, 50));
        assert!(!is_expired(50, 100));
        assert!(!is_expired(100, 0)); // 0 = never expires
    }
}
