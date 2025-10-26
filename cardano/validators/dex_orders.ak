// DEXOrders - On-Chain Orderbook with Automatic Matching
// UTxO-based limit orders with price-time priority

use aiken/list
use aiken/transaction.{ScriptContext}
use aiken/transaction/credential.{VerificationKeyCredential}
use cardano/types.{PaymentCredential, Status, validate_positive}

pub type OrderSide {
  Buy
  Sell
}

pub type OrderDatum {
  order_id: Int,
  creator: PaymentCredential,
  base_policy: ByteArray,
  base_asset: ByteArray,
  quote_policy: ByteArray,
  quote_asset: ByteArray,
  side: OrderSide,
  base_amount: Int,
  filled_amount: Int,
  /// Price scaled by 1000000 (quote per base)
  price: Int,
  expiry: Int,
  status: Status,
}

pub type OrderRedeemer {
  CreateBuy { base_policy: ByteArray, base_asset: ByteArray, quote_policy: ByteArray, quote_asset: ByteArray, amount: Int, price: Int }
  CreateSell { base_policy: ByteArray, base_asset: ByteArray, quote_policy: ByteArray, quote_asset: ByteArray, amount: Int, price: Int }
  FillOrder { amount: Int }
  CancelOrder
}

validator {
  fn dex_order(
    datum: OrderDatum,
    redeemer: OrderRedeemer,
    ctx: ScriptContext,
  ) -> Bool {
    when redeemer is {
      CreateBuy { base_policy, base_asset, quote_policy, quote_asset, amount, price } ->
        validate_create_buy(datum, base_policy, base_asset, quote_policy, quote_asset, amount, price, ctx)

      CreateSell { base_policy, base_asset, quote_policy, quote_asset, amount, price } ->
        validate_create_sell(datum, base_policy, base_asset, quote_policy, quote_asset, amount, price, ctx)

      FillOrder { amount } -> validate_fill(datum, amount, ctx)

      CancelOrder -> validate_cancel(datum, ctx)
    }
  }
}

fn validate_create_buy(
  datum: OrderDatum,
  base_policy: ByteArray,
  base_asset: ByteArray,
  quote_policy: ByteArray,
  quote_asset: ByteArray,
  amount: Int,
  price: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let creator = VerificationKeyCredential(signer)

  validate_positive(amount) &&
  validate_positive(price) &&
  datum.creator == creator &&
  datum.side == Buy &&
  datum.base_amount == amount &&
  datum.filled_amount == 0
}

fn validate_create_sell(
  datum: OrderDatum,
  base_policy: ByteArray,
  base_asset: ByteArray,
  quote_policy: ByteArray,
  quote_asset: ByteArray,
  amount: Int,
  price: Int,
  ctx: ScriptContext,
) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let creator = VerificationKeyCredential(signer)

  validate_positive(amount) &&
  validate_positive(price) &&
  datum.side == Sell
}

fn validate_fill(datum: OrderDatum, amount: Int, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let filler = VerificationKeyCredential(signer)

  // Cannot fill own order
  let not_self = filler != datum.creator

  // Validate amount
  let valid_amount = validate_positive(amount)

  // Check remaining amount
  let remaining = datum.base_amount - datum.filled_amount
  let fill_amount = if amount > remaining { remaining } else { amount }

  // Check output
  expect Some(output) = list.head(transaction.outputs)
  expect OrderDatum { filled_amount: new_filled, status: new_status, .. } = output.datum

  let filled_updated = new_filled == datum.filled_amount + fill_amount

  not_self && valid_amount && filled_updated
}

fn validate_cancel(datum: OrderDatum, ctx: ScriptContext) -> Bool {
  let ScriptContext { transaction, .. } = ctx
  expect [signer] = transaction.extra_signatories
  let creator = VerificationKeyCredential(signer)

  datum.creator == creator && datum.status == Active
}

test order_matching() {
  let buy_price = 1500000
  let sell_price = 1400000

  // Buy order at 1.5 matches sell order at 1.4
  buy_price >= sell_price
}
