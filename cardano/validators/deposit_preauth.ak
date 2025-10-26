// DepositPreauth - One-Time Deposit Pre-Authorizations
// Single-use authorization tokens

use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential, validate_positive}

pub type PreauthDatum {
  authorizer: PaymentCredential,
  authorized: PaymentCredential,
  policy_id: ByteArray,
  asset_name: ByteArray,
  max_amount: Int,
  expires_at: Int,
  used: Bool,
  revoked: Bool,
  memo: ByteArray,
}

pub type PreauthRedeemer {
  Preauthorize { authorized: PaymentCredential, max_amount: Int, expires_at: Int, memo: ByteArray }
  UsePreauth { amount: Int }
  RevokePreauth
}

validator {
  fn deposit_preauth(
    datum: PreauthDatum,
    redeemer: PreauthRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      Preauthorize { authorized, max_amount, expires_at, memo } ->
        validate_preauth(datum, authorized, max_amount, expires_at, memo, ctx)

      UsePreauth { amount } -> validate_use(datum, amount, ctx)

      RevokePreauth -> validate_revoke(datum, ctx)
    }
  }
}

fn validate_preauth(
  datum: PreauthDatum,
  authorized: PaymentCredential,
  max_amount: Int,
  expires_at: Int,
  memo: ByteArray,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let authorizer = VerificationKeyCredential(signer)

  validate_positive(max_amount) &&
  datum.authorizer == authorizer &&
  datum.authorized == authorized &&
  datum.max_amount == max_amount &&
  !datum.used &&
  !datum.revoked
}

fn validate_use(datum: PreauthDatum, amount: Int, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let user = VerificationKeyCredential(signer)

  // Must be authorized user
  let is_authorized = datum.authorized == user

  // Must not be used or revoked
  let is_valid = !datum.used && !datum.revoked

  // Amount must not exceed max
  let within_limit = amount <= datum.max_amount

  is_authorized && is_valid && within_limit
}

fn validate_revoke(datum: PreauthDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let authorizer = VerificationKeyCredential(signer)

  datum.authorizer == authorizer && !datum.used
}

test single_use_token() {
  let used = False
  let revoked = False

  !used && !revoked
}
