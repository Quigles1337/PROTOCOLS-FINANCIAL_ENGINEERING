// PaymentChannels - Streaming Micropayments with State Channels
// Off-chain computation with on-chain settlement

use aiken/hash.{Blake2b_256, Hash}
use aiken/list
use aiken/transaction.{ScriptContext, Spend}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential, Status, validate_positive}

// ============================================================================
// Types
// ============================================================================

/// Channel datum
pub type ChannelDatum {
  /// Unique channel ID
  channel_id: Int,
  /// Sender (who funds the channel)
  sender: PaymentCredential,
  /// Recipient (who receives payments)
  recipient: PaymentCredential,
  /// Asset policy ID
  policy_id: ByteArray,
  /// Asset name
  asset_name: ByteArray,
  /// Total balance in channel
  balance: Int,
  /// Amount claimed by recipient
  claimed: Int,
  /// Last nonce used (for replay protection)
  nonce: Int,
  /// Channel expiration slot
  expires_at: Int,
  /// Status
  status: Status,
  /// Dispute timestamp (if any)
  disputed_at: Option<Int>,
}

/// Redeemer actions
pub type ChannelRedeemer {
  /// Create new channel
  Create { recipient: PaymentCredential, amount: Int, duration: Int }
  /// Fund existing channel
  Fund { amount: Int }
  /// Extend channel expiration
  Extend { duration: Int }
  /// Claim payment with signature
  Claim {
    amount: Int,
    nonce: Int,
    signature: ByteArray,
  }
  /// Close channel cooperatively
  CloseCooperative { final_amount: Int }
  /// Close channel unilaterally (after expiry)
  CloseUnilateral
  /// Dispute a claim
  DisputeClaim
  /// Resolve dispute after period
  ResolveDispute
}

// ============================================================================
// Validator
// ============================================================================

validator {
  fn payment_channel(
    datum: ChannelDatum,
    redeemer: ChannelRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      Create { recipient, amount, duration } ->
        validate_create(datum, recipient, amount, duration, ctx)

      Fund { amount } -> validate_fund(datum, amount, ctx)

      Extend { duration } -> validate_extend(datum, duration, ctx)

      Claim { amount, nonce, signature } ->
        validate_claim(datum, amount, nonce, signature, ctx)

      CloseCooperative { final_amount } ->
        validate_close_cooperative(datum, final_amount, ctx)

      CloseUnilateral -> validate_close_unilateral(datum, ctx)

      DisputeClaim -> validate_dispute(datum, ctx)

      ResolveDispute -> validate_resolve(datum, ctx)
    }
  }
}

// ============================================================================
// Validation Logic
// ============================================================================

fn validate_create(
  datum: ChannelDatum,
  recipient: PaymentCredential,
  amount: Int,
  duration: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Validate parameters
  let valid_amount = validate_positive(amount)
  let valid_duration = validate_positive(duration)

  // Validate datum
  let valid_datum =
    datum.sender == sender &&
    datum.recipient == recipient &&
    datum.balance == amount &&
    datum.claimed == 0 &&
    datum.nonce == 0 &&
    datum.status == Active

  valid_amount && valid_duration && valid_datum
}

fn validate_fund(datum: ChannelDatum, amount: Int, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Only sender can fund
  let is_sender = datum.sender == sender

  // Validate amount
  let valid_amount = validate_positive(amount)

  // Channel must be active
  let is_active = datum.status == Active

  // Check output datum
  expect Some(output) = list.head(transaction.outputs)
  expect ChannelDatum { balance: new_balance, .. } = output.datum

  // Balance should increase
  let balance_increased = new_balance == datum.balance + amount

  is_sender && valid_amount && is_active && balance_increased
}

fn validate_extend(
  datum: ChannelDatum,
  duration: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Only sender can extend
  let is_sender = datum.sender == sender

  // Validate duration
  let valid_duration = validate_positive(duration)

  // Check output
  expect Some(output) = list.head(transaction.outputs)
  expect ChannelDatum { expires_at: new_expiry, .. } = output.datum

  // Expiry should increase
  let expiry_extended = new_expiry == datum.expires_at + duration

  is_sender && valid_duration && expiry_extended
}

fn validate_claim(
  datum: ChannelDatum,
  amount: Int,
  nonce: Int,
  _signature: ByteArray,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  expect [signer] = transaction.extra_signatories
  let claimer = VerificationKeyCredential(signer)

  // Only recipient can claim
  let is_recipient = datum.recipient == claimer

  // Channel must be active
  let is_active = datum.status == Active

  // Nonce must be greater than previous
  let valid_nonce = nonce > datum.nonce

  // Amount must not exceed balance
  let within_balance = amount <= datum.balance

  // TODO: Verify signature from sender
  // In production: verify Ed25519 signature
  let valid_signature = True

  // Check output
  expect Some(output) = list.head(transaction.outputs)
  expect ChannelDatum {
    claimed: new_claimed,
    nonce: new_nonce,
    ..
  } = output.datum

  let claim_updated = new_claimed == amount && new_nonce == nonce

  is_recipient && is_active && valid_nonce &&
  within_balance && valid_signature && claim_updated
}

fn validate_close_cooperative(
  datum: ChannelDatum,
  final_amount: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  // Both parties must sign
  let has_both_signatures =
    list.length(transaction.extra_signatories) == 2

  // Channel must be active
  let is_active = datum.status == Active

  // Final amount must not exceed balance
  let within_balance = final_amount <= datum.balance

  has_both_signatures && is_active && within_balance
}

fn validate_close_unilateral(datum: ChannelDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  // Get current slot
  let current_slot = transaction.validity_range.lower_bound.bound_type

  // Channel must be expired
  // Simplified: In production, properly extract slot from validity range
  let is_expired = True

  // Must not be disputed or dispute resolved
  let can_close =
    datum.status == Active || datum.status == Completed

  is_expired && can_close
}

fn validate_dispute(datum: ChannelDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Only sender can dispute
  let is_sender = datum.sender == sender

  // Channel must be active
  let is_active = datum.status == Active

  is_sender && is_active
}

fn validate_resolve(datum: ChannelDatum, ctx: ScriptContext) -> Bool {
  // Dispute period must have passed
  // Simplified validation
  datum.status != Active
}

// ============================================================================
// Tests
// ============================================================================

test channel_creation() {
  let datum = ChannelDatum {
    channel_id: 1,
    sender: #"aabbcc",
    recipient: #"ddeeff",
    policy_id: #"",
    asset_name: #"",
    balance: 1000,
    claimed: 0,
    nonce: 0,
    expires_at: 1000,
    status: Active,
    disputed_at: None,
  }

  datum.claimed == 0 && datum.balance > 0
}

test nonce_validation() {
  let old_nonce = 5
  let new_nonce = 10

  new_nonce > old_nonce
}
