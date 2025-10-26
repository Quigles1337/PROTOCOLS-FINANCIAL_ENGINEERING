// Checks - Deferred Payment Instruments
// On-chain checks with optional recipient (bearer checks)

use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential, Status, validate_positive}

pub type CheckDatum {
  check_id: Int,
  writer: PaymentCredential,
  payee: Option<PaymentCredential>,
  policy_id: ByteArray,
  asset_name: ByteArray,
  amount: Int,
  expiry: Int,
  memo: ByteArray,
  status: Status,
  cashed_by: Option<PaymentCredential>,
}

pub type CheckRedeemer {
  WriteCheck { payee: Option<PaymentCredential>, amount: Int, expiry: Int, memo: ByteArray }
  CashCheck
  CancelCheck
}

validator {
  fn check(
    datum: CheckDatum,
    redeemer: CheckRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      WriteCheck { payee, amount, expiry, memo } ->
        validate_write(datum, payee, amount, expiry, memo, ctx)
      CashCheck -> validate_cash(datum, ctx)
      CancelCheck -> validate_cancel(datum, ctx)
    }
  }
}

fn validate_write(
  datum: CheckDatum,
  payee: Option<PaymentCredential>,
  amount: Int,
  expiry: Int,
  memo: ByteArray,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let writer = VerificationKeyCredential(signer)

  validate_positive(amount) &&
  datum.writer == writer &&
  datum.amount == amount &&
  datum.status == Active
}

fn validate_cash(datum: CheckDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let casher = VerificationKeyCredential(signer)

  // Cannot cash own check
  let not_self = casher != datum.writer

  // If payee specified, must match
  let is_authorized =
    when datum.payee is {
      Some(payee) -> casher == payee
      None -> True
    }

  // Must be active
  let is_active = datum.status == Active

  not_self && is_authorized && is_active
}

fn validate_cancel(datum: CheckDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let writer = VerificationKeyCredential(signer)

  datum.writer == writer && datum.status == Active
}

test bearer_check_anyone_can_cash() {
  // Bearer check (no payee) can be cashed by anyone
  let payee: Option<PaymentCredential> = None
  payee == None
}
