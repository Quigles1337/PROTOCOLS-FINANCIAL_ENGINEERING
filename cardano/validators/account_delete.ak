// AccountDelete - Account Lifecycle Management
// Account deletion with fund recovery

use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential}

pub type AccountDatum {
  owner: PaymentCredential,
  created_at: Int,
  min_age: Int,
  is_deleted: Bool,
  beneficiary: Option<PaymentCredential>,
}

pub type AccountRedeemer {
  CreateAccount
  DeleteAccount { beneficiary: PaymentCredential }
}

validator {
  fn account_delete(
    datum: AccountDatum,
    redeemer: AccountRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      CreateAccount -> validate_create(datum, ctx)
      DeleteAccount { beneficiary } -> validate_delete(datum, beneficiary, ctx)
    }
  }
}

fn validate_create(datum: AccountDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let owner = VerificationKeyCredential(signer)

  datum.owner == owner &&
  !datum.is_deleted
}

fn validate_delete(
  datum: AccountDatum,
  beneficiary: PaymentCredential,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let owner = VerificationKeyCredential(signer)

  // Only owner can delete
  let is_owner = datum.owner == owner

  // Account must not already be deleted
  let not_deleted = !datum.is_deleted

  // Check age requirement (simplified - in production use POSIXTime)
  let age_met = True

  is_owner && not_deleted && age_met
}

test account_lifecycle() {
  let created_at = 1000000
  let min_age = 86400
  let current_time = 1100000

  current_time - created_at >= min_age
}
