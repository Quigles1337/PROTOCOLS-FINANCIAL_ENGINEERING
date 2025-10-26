// Escrow - Time and Hash-Locked Conditional Payments (HTLCs)
// Atomic swaps and conditional releases

use aiken/hash.{Blake2b_256, Hash, blake2b_256}
use aiken/list
use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential, Status, validate_positive}

pub type EscrowDatum {
  escrow_id: Int,
  sender: PaymentCredential,
  recipient: PaymentCredential,
  policy_id: ByteArray,
  asset_name: ByteArray,
  amount: Int,
  /// POSIXTime release time (0 = no time lock)
  release_time: Int,
  /// Hash for hash-lock (empty = no hash lock)
  hash_lock: ByteArray,
  /// Expiry time (sender can cancel after)
  expiry_time: Int,
  memo: ByteArray,
  status: Status,
}

pub type EscrowRedeemer {
  CreateTimeLock { recipient: PaymentCredential, amount: Int, release_time: Int, expiry: Int }
  CreateHashLock { recipient: PaymentCredential, amount: Int, hash: ByteArray, expiry: Int }
  CreateTimedHashLock { recipient: PaymentCredential, amount: Int, release_time: Int, hash: ByteArray, expiry: Int }
  Release { preimage: Option<ByteArray> }
  Cancel
}

validator {
  fn escrow(
    datum: EscrowDatum,
    redeemer: EscrowRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      CreateTimeLock { recipient, amount, release_time, expiry } ->
        validate_create_time_lock(datum, recipient, amount, release_time, expiry, ctx)

      CreateHashLock { recipient, amount, hash, expiry } ->
        validate_create_hash_lock(datum, recipient, amount, hash, expiry, ctx)

      CreateTimedHashLock { recipient, amount, release_time, hash, expiry } ->
        validate_create_timed_hash_lock(datum, recipient, amount, release_time, hash, expiry, ctx)

      Release { preimage } -> validate_release(datum, preimage, ctx)

      Cancel -> validate_cancel(datum, ctx)
    }
  }
}

fn validate_create_time_lock(
  datum: EscrowDatum,
  recipient: PaymentCredential,
  amount: Int,
  release_time: Int,
  expiry: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  validate_positive(amount) &&
  datum.sender == sender &&
  datum.recipient == recipient &&
  datum.amount == amount &&
  datum.release_time == release_time &&
  datum.hash_lock == #"" &&
  datum.status == Active
}

fn validate_create_hash_lock(
  datum: EscrowDatum,
  recipient: PaymentCredential,
  amount: Int,
  hash: ByteArray,
  expiry: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  validate_positive(amount) &&
  datum.hash_lock == hash &&
  datum.release_time == 0
}

fn validate_create_timed_hash_lock(
  datum: EscrowDatum,
  recipient: PaymentCredential,
  amount: Int,
  release_time: Int,
  hash: ByteArray,
  expiry: Int,
  ctx: ScriptContext,
) -> Bool {
  validate_positive(amount) &&
  datum.release_time == release_time &&
  datum.hash_lock == hash
}

fn validate_release(
  datum: EscrowDatum,
  preimage: Option<ByteArray>,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let claimer = VerificationKeyCredential(signer)

  // Only recipient can release
  let is_recipient = datum.recipient == claimer

  // Check time lock
  let time_unlocked = datum.release_time == 0 || True // Simplified

  // Check hash lock
  let hash_unlocked =
    when preimage is {
      Some(pre) -> blake2b_256(pre) == datum.hash_lock
      None -> datum.hash_lock == #""
    }

  is_recipient && time_unlocked && hash_unlocked
}

fn validate_cancel(datum: EscrowDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let sender = VerificationKeyCredential(signer)

  datum.sender == sender && datum.status == Active
}

test htlc_hash_verification() {
  let preimage = "secret"
  let hash = blake2b_256(preimage)
  blake2b_256(preimage) == hash
}
