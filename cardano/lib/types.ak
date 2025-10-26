// Shared types for XRPL Financial Primitives on Cardano

use aiken/hash.{Blake2b_224, Hash}
use aiken/transaction/credential.{VerificationKey}

/// Payment credential (public key hash)
pub type PaymentCredential =
  Hash<Blake2b_224, VerificationKey>

/// Asset identifier
pub type AssetId {
  policy_id: ByteArray,
  asset_name: ByteArray,
}

/// Common status enum
pub type Status {
  Active
  Completed
  Cancelled
}

/// Calculate percentage with precision
pub fn calculate_percentage(amount: Int, percentage: Int) -> Int {
  amount * percentage / 100
}

/// Check if timestamp has expired
pub fn is_expired(current_time: Int, expiry: Int) -> Bool {
  expiry > 0 && current_time >= expiry
}

/// Validate amount is positive
pub fn validate_positive(amount: Int) -> Bool {
  amount > 0
}
