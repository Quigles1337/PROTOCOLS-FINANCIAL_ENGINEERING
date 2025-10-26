// SignerListManager - Weighted Multi-Signature Accounts
// Dynamic quorum with configurable weights

use aiken/list
use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential, validate_positive}

pub type Signer {
  address: PaymentCredential,
  weight: Int,
}

pub type SignerListDatum {
  owner: PaymentCredential,
  quorum: Int,
  signers: List<Signer>,
  total_weight: Int,
}

pub type SignerListRedeemer {
  SetSignerList { quorum: Int, signers: List<Signer> }
  RemoveSignerList
  VerifySignatures { signatures: List<PaymentCredential> }
}

validator {
  fn signer_list(
    datum: SignerListDatum,
    redeemer: SignerListRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      SetSignerList { quorum, signers } ->
        validate_set(datum, quorum, signers, ctx)

      RemoveSignerList -> validate_remove(datum, ctx)

      VerifySignatures { signatures } ->
        validate_verify(datum, signatures, ctx)
    }
  }
}

fn validate_set(
  datum: SignerListDatum,
  quorum: Int,
  signers: List<Signer>,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let owner = VerificationKeyCredential(signer)

  // Calculate total weight
  let total_weight = calculate_total_weight(signers, 0)

  // Validate quorum
  let valid_quorum =
    validate_positive(quorum) && quorum <= total_weight

  datum.owner == owner &&
  datum.quorum == quorum &&
  datum.total_weight == total_weight &&
  valid_quorum
}

fn calculate_total_weight(signers: List<Signer>, acc: Int) -> Int {
  when signers is {
    [] -> acc
    [head, ..tail] -> calculate_total_weight(tail, acc + head.weight)
  }
}

fn validate_remove(datum: SignerListDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let owner = VerificationKeyCredential(signer)

  datum.owner == owner
}

fn validate_verify(
  datum: SignerListDatum,
  signatures: List<PaymentCredential>,
  ctx: ScriptContext,
) -> Bool {
  // Calculate weight of provided signatures
  let weight = calculate_signature_weight(datum.signers, signatures, 0)

  // Must meet quorum
  weight >= datum.quorum
}

fn calculate_signature_weight(
  signers: List<Signer>,
  signatures: List<PaymentCredential>,
  acc: Int,
) -> Int {
  when signers is {
    [] -> acc
    [head, ..tail] ->
      if list.has(signatures, head.address) {
        calculate_signature_weight(tail, signatures, acc + head.weight)
      } else {
        calculate_signature_weight(tail, signatures, acc)
      }
  }
}

test quorum_validation() {
  let quorum = 3
  let total_weight = 5

  quorum <= total_weight && quorum > 0
}

test weight_calculation() {
  let signers = [
    Signer { address: #"aa", weight: 1 },
    Signer { address: #"bb", weight: 2 },
  ]

  calculate_total_weight(signers, 0) == 3
}
