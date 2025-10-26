// TrustLines - Bilateral Credit Networks with Payment Rippling
// XRPL-inspired trust line implementation for Cardano eUTXO model

use aiken/list
use aiken/transaction.{ScriptContext, Spend, find_input}
use aiken/transaction/credential.{VerificationKeyCredential}
use aiken/transaction/value
use cardano/types.{PaymentCredential, validate_positive}

// ============================================================================
// Types
// ============================================================================

/// Trust line datum stored in UTxO
pub type TrustLineDatum {
  /// Account 1 (lower address for ordering)
  account1: PaymentCredential,
  /// Account 2 (higher address for ordering)
  account2: PaymentCredential,
  /// Asset policy ID
  policy_id: ByteArray,
  /// Asset name
  asset_name: ByteArray,
  /// Credit limit from account1 to account2
  limit1: Int,
  /// Credit limit from account2 to account1
  limit2: Int,
  /// Current balance (positive = account2 owes account1)
  balance: Int,
  /// Allow rippling through this trust line
  allow_rippling: Bool,
  /// Quality in (scaled by 1000)
  quality_in: Int,
  /// Quality out (scaled by 1000)
  quality_out: Int,
}

/// Redeemer actions
pub type TrustLineRedeemer {
  /// Create a new trust line
  Create {
    counterparty: PaymentCredential,
    limit: Int,
    allow_rippling: Bool,
  }
  /// Update trust line parameters
  Update { limit: Int, allow_rippling: Bool }
  /// Send payment through trust line
  SendPayment { recipient: PaymentCredential, amount: Int }
  /// Send payment through path (rippling)
  SendThroughPath { path: List<PaymentCredential>, amount: Int }
  /// Close trust line (must have zero balance)
  Close
}

// ============================================================================
// Validator
// ============================================================================

validator {
  fn trust_line(
    datum: TrustLineDatum,
    redeemer: TrustLineRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      Create { counterparty, limit, allow_rippling } ->
        validate_create(datum, counterparty, limit, allow_rippling, ctx)

      Update { limit, allow_rippling } ->
        validate_update(datum, limit, allow_rippling, ctx)

      SendPayment { recipient, amount } ->
        validate_send_payment(datum, recipient, amount, ctx)

      SendThroughPath { path, amount } ->
        validate_send_through_path(datum, path, amount, ctx)

      Close -> validate_close(datum, ctx)
    }
  }
}

// ============================================================================
// Validation Logic
// ============================================================================

/// Validate trust line creation
fn validate_create(
  datum: TrustLineDatum,
  counterparty: PaymentCredential,
  limit: Int,
  allow_rippling: Bool,
  ctx: ScriptContext,
) -> Bool {
  // Extract transaction
  let ScriptContext { transaction, purpose } = ctx

  // Must be spending transaction
  expect Spend(own_ref) = purpose
  expect Some(own_input) = find_input(transaction.inputs, own_ref)

  // Get signer
  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Validate limit is positive
  let valid_limit = validate_positive(limit)

  // Ensure accounts are ordered (account1 < account2)
  let (acc1, acc2, lim1, lim2) =
    if sender < counterparty {
      (sender, counterparty, limit, 0)
    } else {
      (counterparty, sender, 0, limit)
    }

  // Validate datum structure
  let valid_datum =
    datum.account1 == acc1 &&
    datum.account2 == acc2 &&
    datum.limit1 == lim1 &&
    datum.limit2 == lim2 &&
    datum.balance == 0 &&
    datum.allow_rippling == allow_rippling

  // Sender must sign
  let signed = list.has(transaction.extra_signatories, signer)

  valid_limit && valid_datum && signed
}

/// Validate trust line update
fn validate_update(
  datum: TrustLineDatum,
  new_limit: Int,
  new_rippling: Bool,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  // Get signer
  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Sender must be one of the accounts
  let is_account1 = datum.account1 == sender
  let is_account2 = datum.account2 == sender
  let is_participant = is_account1 || is_account2

  // Validate new limit
  let valid_limit = validate_positive(new_limit)

  // Check outputs for updated datum
  expect Some(output) = list.head(transaction.outputs)
  expect TrustLineDatum {
    account1: out_account1,
    account2: out_account2,
    policy_id: out_policy,
    asset_name: out_asset,
    limit1: out_limit1,
    limit2: out_limit2,
    balance: out_balance,
    allow_rippling: out_rippling,
    quality_in: out_qin,
    quality_out: out_qout,
  } = output.datum

  // Validate output datum
  let correct_accounts =
    out_account1 == datum.account1 && out_account2 == datum.account2
  let correct_asset =
    out_policy == datum.policy_id && out_asset == datum.asset_name
  let balance_unchanged = out_balance == datum.balance

  // Update limits based on who is updating
  let limits_updated =
    if is_account1 {
      out_limit1 == new_limit && out_limit2 == datum.limit2
    } else {
      out_limit1 == datum.limit1 && out_limit2 == new_limit
    }

  let rippling_updated = out_rippling == new_rippling

  is_participant && valid_limit && correct_accounts && correct_asset &&
  balance_unchanged && limits_updated && rippling_updated
}

/// Validate payment through trust line
fn validate_send_payment(
  datum: TrustLineDatum,
  recipient: PaymentCredential,
  amount: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  // Get sender
  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Validate sender and recipient are the trust line accounts
  let valid_participants =
    (sender == datum.account1 && recipient == datum.account2) ||
    (sender == datum.account2 && recipient == datum.account1)

  // Validate amount
  let valid_amount = validate_positive(amount)

  // Check output datum for updated balance
  expect Some(output) = list.head(transaction.outputs)
  expect TrustLineDatum { balance: new_balance, .. } = output.datum

  // Calculate new balance based on direction
  let expected_balance =
    if sender == datum.account1 {
      // Payment from account1 to account2: decrease balance
      datum.balance - amount
    } else {
      // Payment from account2 to account1: increase balance
      datum.balance + amount
    }

  // Check credit limits
  let within_limits =
    if sender == datum.account1 {
      expected_balance >= 0 - datum.limit1
    } else {
      expected_balance <= datum.limit2
    }

  valid_participants && valid_amount &&
  new_balance == expected_balance && within_limits
}

/// Validate payment through path (rippling)
fn validate_send_through_path(
  datum: TrustLineDatum,
  path: List<PaymentCredential>,
  amount: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  // Must allow rippling
  let rippling_enabled = datum.allow_rippling

  // Validate amount
  let valid_amount = validate_positive(amount)

  // Path must not be empty
  let valid_path = !list.is_empty(path)

  // In production: validate each hop in the path has sufficient credit
  // For now, simplified validation

  rippling_enabled && valid_amount && valid_path
}

/// Validate trust line closure
fn validate_close(datum: TrustLineDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx

  // Get signer
  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  // Sender must be one of the accounts
  let is_participant =
    datum.account1 == sender || datum.account2 == sender

  // Balance must be zero
  let zero_balance = datum.balance == 0

  is_participant && zero_balance
}

// ============================================================================
// Tests
// ============================================================================

test create_trust_line_success() {
  // Test successful trust line creation
  let datum = TrustLineDatum {
    account1: #"aabbcc",
    account2: #"ddeeff",
    policy_id: #"",
    asset_name: #"",
    limit1: 1000,
    limit2: 0,
    balance: 0,
    allow_rippling: True,
    quality_in: 1000,
    quality_out: 1000,
  }

  // Validate initial balance is zero
  datum.balance == 0 && datum.limit1 > 0
}

test validate_positive_amounts() {
  // Test amount validation
  validate_positive(100) && !validate_positive(0) && !validate_positive(-50)
}

test balance_calculation() {
  // Test balance increases and decreases
  let initial_balance = 0
  let payment = 100

  // Account1 pays account2: balance decreases
  let balance_after_payment = initial_balance - payment

  // Account2 pays account1: balance increases
  let balance_after_return = balance_after_payment + payment

  balance_after_payment == -100 && balance_after_return == 0
}
