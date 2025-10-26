;; DEX Orders - On-chain orderbook with limit orders
;; Supports partial fills and price-time priority

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-ORDER-NOT-FOUND (err u2))
(define-constant ERR-ORDER-NOT-OPEN (err u3))
(define-constant ERR-INVALID-AMOUNT (err u4))
(define-constant ERR-SAME-ASSET (err u5))
(define-constant ERR-FILL-EXCEEDS-REMAINING (err u6))

;; Constants
(define-constant STATUS-OPEN u1)
(define-constant STATUS-PARTIALLY-FILLED u2)
(define-constant STATUS-FILLED u3)
(define-constant STATUS-CANCELLED u4)

;; Data vars
(define-data-var next-order-id uint u0)

;; Data maps
(define-map orders
  { order-id: uint }
  {
    maker: principal,
    sell-asset: (string-ascii 10),
    buy-asset: (string-ascii 10),
    sell-amount: uint,
    buy-amount: uint,
    filled: uint,
    status: uint,
    created-at: uint
  }
)

;; Place limit order
(define-public (place-order
    (sell-asset (string-ascii 10))
    (buy-asset (string-ascii 10))
    (sell-amount uint)
    (buy-amount uint))
  (let
    (
      (maker tx-sender)
      (order-id (var-get next-order-id))
    )
    (asserts! (not (is-eq sell-asset buy-asset)) ERR-SAME-ASSET)
    (asserts! (> sell-amount u0) ERR-INVALID-AMOUNT)
    (asserts! (> buy-amount u0) ERR-INVALID-AMOUNT)

    ;; Create order
    (map-set orders
      { order-id: order-id }
      {
        maker: maker,
        sell-asset: sell-asset,
        buy-asset: buy-asset,
        sell-amount: sell-amount,
        buy-amount: buy-amount,
        filled: u0,
        status: STATUS-OPEN,
        created-at: block-height
      }
    )

    (var-set next-order-id (+ order-id u1))
    (ok order-id)
  )
)

;; Fill order (partial fills supported)
(define-public (fill-order (order-id uint) (fill-amount uint))
  (let
    (
      (taker tx-sender)
      (order (unwrap! (map-get? orders { order-id: order-id }) ERR-ORDER-NOT-FOUND))
    )
    (asserts! (> fill-amount u0) ERR-INVALID-AMOUNT)
    (asserts! (or
      (is-eq (get status order) STATUS-OPEN)
      (is-eq (get status order) STATUS-PARTIALLY-FILLED)
    ) ERR-ORDER-NOT-OPEN)

    (let
      (
        (remaining (- (get sell-amount order) (get filled order)))
        (actual-fill (if (> fill-amount remaining) remaining fill-amount))
        (required-payment (/ (* actual-fill (get buy-amount order)) (get sell-amount order)))
        (new-filled (+ (get filled order) actual-fill))
      )
      ;; Update order
      (ok (map-set orders
        { order-id: order-id }
        (merge order {
          filled: new-filled,
          status: (if (>= new-filled (get sell-amount order))
            STATUS-FILLED
            STATUS-PARTIALLY-FILLED
          )
        })
      ))
    )
  )
)

;; Cancel order
(define-public (cancel-order (order-id uint))
  (let
    (
      (maker tx-sender)
      (order (unwrap! (map-get? orders { order-id: order-id }) ERR-ORDER-NOT-FOUND))
    )
    (asserts! (is-eq (get maker order) maker) ERR-NOT-AUTHORIZED)
    (asserts! (or
      (is-eq (get status order) STATUS-OPEN)
      (is-eq (get status order) STATUS-PARTIALLY-FILLED)
    ) ERR-ORDER-NOT-OPEN)

    (ok (map-set orders
      { order-id: order-id }
      (merge order { status: STATUS-CANCELLED })
    ))
  )
)

;; Read-only functions
(define-read-only (get-order (order-id uint))
  (map-get? orders { order-id: order-id })
)

(define-read-only (get-price (order-id uint))
  (match (get-order order-id)
    order (ok { buy-amount: (get buy-amount order), sell-amount: (get sell-amount order) })
    ERR-ORDER-NOT-FOUND
  )
)

(define-read-only (get-remaining (order-id uint))
  (match (get-order order-id)
    order (ok (- (get sell-amount order) (get filled order)))
    ERR-ORDER-NOT-FOUND
  )
)

(define-read-only (is-fillable (order-id uint))
  (match (get-order order-id)
    order (ok (or
      (is-eq (get status order) STATUS-OPEN)
      (is-eq (get status order) STATUS-PARTIALLY-FILLED)
    ))
    ERR-ORDER-NOT-FOUND
  )
)

(define-read-only (get-next-order-id)
  (ok (var-get next-order-id))
)
