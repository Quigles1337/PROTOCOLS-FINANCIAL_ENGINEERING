// DepositAuthorization - KYC/AML Compliance System
// Whitelist/blacklist mode for deposits

use aiken/list
use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential}

pub type AuthMode {
  Whitelist
  Blacklist
}

pub type AuthDatum {
  account: PaymentCredential,
  enabled: Bool,
  mode: AuthMode,
  authorized: List<PaymentCredential>,
  require_kyc: Bool,
}

pub type AuthRedeemer {
  EnableAuth { mode: AuthMode }
  DisableAuth
  AuthorizeDepositor { depositor: PaymentCredential }
  UnauthorizeDepositor { depositor: PaymentCredential }
  CheckDeposit { depositor: PaymentCredential }
}

validator {
  fn deposit_auth(
    datum: AuthDatum,
    redeemer: AuthRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      EnableAuth { mode } -> validate_enable(datum, mode, ctx)
      DisableAuth -> validate_disable(datum, ctx)
      AuthorizeDepositor { depositor } -> validate_authorize(datum, depositor, ctx)
      UnauthorizeDepositor { depositor } -> validate_unauthorize(datum, depositor, ctx)
      CheckDeposit { depositor } -> validate_check(datum, depositor, ctx)
    }
  }
}

fn validate_enable(datum: AuthDatum, mode: AuthMode, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let account = VerificationKeyCredential(signer)

  datum.account == account && datum.enabled
}

fn validate_disable(datum: AuthDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let account = VerificationKeyCredential(signer)

  datum.account == account
}

fn validate_authorize(
  datum: AuthDatum,
  depositor: PaymentCredential,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let account = VerificationKeyCredential(signer)

  // Check output has depositor in list
  expect Some(output) = list.head(transaction.outputs)
  expect AuthDatum { authorized: new_authorized, .. } = output.datum

  datum.account == account &&
  list.has(new_authorized, depositor)
}

fn validate_unauthorize(
  datum: AuthDatum,
  depositor: PaymentCredential,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let account = VerificationKeyCredential(signer)

  datum.account == account &&
  list.has(datum.authorized, depositor)
}

fn validate_check(
  datum: AuthDatum,
  depositor: PaymentCredential,
  ctx: ScriptContext,
) -> Bool {
  if !datum.enabled {
    True
  } else {
    let is_on_list = list.has(datum.authorized, depositor)

    when datum.mode is {
      Whitelist -> is_on_list
      Blacklist -> !is_on_list
    }
  }
}

test whitelist_mode() {
  let authorized = [#"aabbcc", #"ddeeff"]
  list.has(authorized, #"aabbcc")
}
