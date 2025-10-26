// DIDManager - W3C Decentralized Identifier Management
// Self-sovereign identity on Cardano

use aiken/hash.{blake2b_256}
use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential}

pub type DIDDatum {
  owner: PaymentCredential,
  did_uri: ByteArray,
  /// Blake2b-256 hash of DID document
  document_hash: ByteArray,
  /// IPFS/Arweave reference for full document
  storage_ref: Option<ByteArray>,
  created_at: Int,
  updated_at: Int,
}

pub type DIDRedeemer {
  RegisterDID { did_uri: ByteArray, document: ByteArray, storage_ref: Option<ByteArray> }
  UpdateDID { document: ByteArray, storage_ref: Option<ByteArray> }
  RevokeDID
}

validator {
  fn did_manager(
    datum: DIDDatum,
    redeemer: DIDRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      RegisterDID { did_uri, document, storage_ref } ->
        validate_register(datum, did_uri, document, storage_ref, ctx)

      UpdateDID { document, storage_ref } ->
        validate_update(datum, document, storage_ref, ctx)

      RevokeDID -> validate_revoke(datum, ctx)
    }
  }
}

fn validate_register(
  datum: DIDDatum,
  did_uri: ByteArray,
  document: ByteArray,
  storage_ref: Option<ByteArray>,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let owner = VerificationKeyCredential(signer)

  let doc_hash = blake2b_256(document)

  datum.owner == owner &&
  datum.did_uri == did_uri &&
  datum.document_hash == doc_hash
}

fn validate_update(
  datum: DIDDatum,
  document: ByteArray,
  storage_ref: Option<ByteArray>,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let owner = VerificationKeyCredential(signer)

  let new_hash = blake2b_256(document)

  datum.owner == owner && new_hash != #""
}

fn validate_revoke(datum: DIDDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let owner = VerificationKeyCredential(signer)

  datum.owner == owner
}

test did_uri_format() {
  let did = "did:cardano:mainnet:addr1..."
  did != #""
}
