;; TrustLines - Bilateral credit lines with payment rippling
;; Enables credit networks with multi-hop payment routing

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-TRUST-LINE-EXISTS (err u2))
(define-constant ERR-TRUST-LINE-NOT-FOUND (err u3))
(define-constant ERR-SAME-ACCOUNT (err u4))
(define-constant ERR-BALANCE-EXCEEDED (err u5))
(define-constant ERR-BALANCE-NOT-ZERO (err u6))
(define-constant ERR-INVALID-AMOUNT (err u7))

;; Data maps
(define-map trust-lines
  { account1: principal, account2: principal }
  {
    limit1: uint,         ;; Credit limit account1 extends to account2
    limit2: uint,         ;; Credit limit account2 extends to account1
    balance: uint,        ;; Current balance
    is-negative: bool,    ;; True if balance is negative
    active: bool,
    created-at: uint
  }
)

;; Helper: Order addresses consistently for map key
(define-private (order-addresses (addr1 principal) (addr2 principal))
  (if (is-eq addr1 addr2)
    { account1: addr1, account2: addr2 }
    (if (< (buff-to-uint-be (unwrap-panic (to-consensus-buff? addr1)))
           (buff-to-uint-be (unwrap-panic (to-consensus-buff? addr2))))
      { account1: addr1, account2: addr2 }
      { account1: addr2, account2: addr1 }
    )
  )
)

;; Create trust line
(define-public (create-trust-line (counterparty principal) (limit1 uint) (limit2 uint))
  (let
    (
      (sender tx-sender)
      (ordered (order-addresses sender counterparty))
    )
    (asserts! (not (is-eq sender counterparty)) ERR-SAME-ACCOUNT)
    (asserts! (is-none (map-get? trust-lines ordered)) ERR-TRUST-LINE-EXISTS)

    (ok (map-set trust-lines
      ordered
      {
        limit1: limit1,
        limit2: limit2,
        balance: u0,
        is-negative: false,
        active: true,
        created-at: block-height
      }
    ))
  )
)

;; Update credit limit
(define-public (update-limit (counterparty principal) (new-limit uint))
  (let
    (
      (sender tx-sender)
      (ordered (order-addresses sender counterparty))
      (trust-line (unwrap! (map-get? trust-lines ordered) ERR-TRUST-LINE-NOT-FOUND))
    )
    (asserts! (get active trust-line) ERR-TRUST-LINE-NOT-FOUND)

    ;; Update the appropriate limit based on which account is calling
    (ok (if (is-eq (get account1 ordered) sender)
      (map-set trust-lines ordered (merge trust-line { limit1: new-limit }))
      (map-set trust-lines ordered (merge trust-line { limit2: new-limit }))
    ))
  )
)

;; Ripple payment through trust line
(define-public (ripple-payment (receiver principal) (amount uint))
  (let
    (
      (sender tx-sender)
      (ordered (order-addresses sender receiver))
      (trust-line (unwrap! (map-get? trust-lines ordered) ERR-TRUST-LINE-NOT-FOUND))
    )
    (asserts! (> amount u0) ERR-INVALID-AMOUNT)
    (asserts! (get active trust-line) ERR-TRUST-LINE-NOT-FOUND)

    ;; Calculate new balance based on direction
    (let
      (
        (is-account1 (is-eq (get account1 ordered) sender))
        (current-balance (get balance trust-line))
        (is-neg (get is-negative trust-line))
      )
      (if is-account1
        ;; account1 -> account2: increase balance
        (if is-neg
          ;; Balance was negative, reduce it
          (if (>= amount current-balance)
            (ok (map-set trust-lines ordered (merge trust-line {
              balance: (- amount current-balance),
              is-negative: false
            })))
            (ok (map-set trust-lines ordered (merge trust-line {
              balance: (- current-balance amount)
            })))
          )
          ;; Balance positive, increase it
          (begin
            (asserts! (<= (+ current-balance amount) (get limit1 trust-line)) ERR-BALANCE-EXCEEDED)
            (ok (map-set trust-lines ordered (merge trust-line {
              balance: (+ current-balance amount)
            })))
          )
        )
        ;; account2 -> account1: decrease balance (or make negative)
        (if (not is-neg)
          (if (>= amount current-balance)
            (begin
              (asserts! (<= (- amount current-balance) (get limit2 trust-line)) ERR-BALANCE-EXCEEDED)
              (ok (map-set trust-lines ordered (merge trust-line {
                balance: (- amount current-balance),
                is-negative: true
              })))
            )
            (ok (map-set trust-lines ordered (merge trust-line {
              balance: (- current-balance amount)
            })))
          )
          ;; Already negative, increase negative balance
          (begin
            (asserts! (<= (+ current-balance amount) (get limit2 trust-line)) ERR-BALANCE-EXCEEDED)
            (ok (map-set trust-lines ordered (merge trust-line {
              balance: (+ current-balance amount)
            })))
          )
        )
      )
    )
  )
)

;; Close trust line (only if balance is zero)
(define-public (close-trust-line (counterparty principal))
  (let
    (
      (sender tx-sender)
      (ordered (order-addresses sender counterparty))
      (trust-line (unwrap! (map-get? trust-lines ordered) ERR-TRUST-LINE-NOT-FOUND))
    )
    (asserts! (is-eq (get balance trust-line) u0) ERR-BALANCE-NOT-ZERO)
    (ok (map-set trust-lines ordered (merge trust-line { active: false })))
  )
)

;; Read-only functions
(define-read-only (get-trust-line (account1 principal) (account2 principal))
  (let
    (
      (ordered (order-addresses account1 account2))
    )
    (map-get? trust-lines ordered)
  )
)

(define-read-only (trust-line-exists (account1 principal) (account2 principal))
  (is-some (get-trust-line account1 account2))
)

(define-read-only (get-balance (account1 principal) (account2 principal))
  (match (get-trust-line account1 account2)
    trust-line (ok {
      balance: (get balance trust-line),
      is-negative: (get is-negative trust-line)
    })
    ERR-TRUST-LINE-NOT-FOUND
  )
)

;; Helper to convert buffer to uint (simplified)
(define-private (buff-to-uint-be (buff (buff 128)))
  (fold + (map buff-to-uint-8 (list buff)) u0)
)

(define-private (buff-to-uint-8 (byte (buff 1)))
  (buff-to-uint-le byte)
)
