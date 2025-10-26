;; Checks - Deferred payments like paper checks
;; Recipients can cash checks later, with partial cashing support

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-CHECK-NOT-FOUND (err u2))
(define-constant ERR-CHECK-EXPIRED (err u3))
(define-constant ERR-CHECK-CASHED (err u4))
(define-constant ERR-INVALID-AMOUNT (err u5))
(define-constant ERR-AMOUNT-EXCEEDS-CHECK (err u6))

;; Constants
(define-constant STATUS-ACTIVE u1)
(define-constant STATUS-CASHED u2)
(define-constant STATUS-CANCELLED u3)

;; Data vars
(define-data-var next-check-id uint u0)

;; Data maps
(define-map checks
  { check-id: uint }
  {
    sender: principal,
    receiver: principal,
    amount: uint,
    expiration: uint,
    status: uint,
    cashed-amount: uint,
    created-at: uint
  }
)

;; Create check
(define-public (create-check (receiver principal) (amount uint) (expiration uint))
  (let
    (
      (sender tx-sender)
      (check-id (var-get next-check-id))
    )
    (asserts! (> amount u0) ERR-INVALID-AMOUNT)
    (asserts! (> expiration block-height) ERR-CHECK-EXPIRED)

    ;; Transfer STX to contract
    (try! (stx-transfer? amount sender (as-contract tx-sender)))

    ;; Create check
    (map-set checks
      { check-id: check-id }
      {
        sender: sender,
        receiver: receiver,
        amount: amount,
        expiration: expiration,
        status: STATUS-ACTIVE,
        cashed-amount: u0,
        created-at: block-height
      }
    )

    (var-set next-check-id (+ check-id u1))
    (ok check-id)
  )
)

;; Cash check (receiver only, supports partial amounts)
(define-public (cash-check (check-id uint) (amount uint))
  (let
    (
      (receiver tx-sender)
      (check (unwrap! (map-get? checks { check-id: check-id }) ERR-CHECK-NOT-FOUND))
    )
    (asserts! (is-eq (get receiver check) receiver) ERR-NOT-AUTHORIZED)
    (asserts! (is-eq (get status check) STATUS-ACTIVE) ERR-CHECK-CASHED)
    (asserts! (< block-height (get expiration check)) ERR-CHECK-EXPIRED)
    (asserts! (> amount u0) ERR-INVALID-AMOUNT)

    (let
      (
        (remaining (- (get amount check) (get cashed-amount check)))
      )
      (asserts! (<= amount remaining) ERR-AMOUNT-EXCEEDS-CHECK)

      ;; Transfer STX to receiver
      (try! (as-contract (stx-transfer? amount tx-sender receiver)))

      (let
        (
          (new-cashed-amount (+ (get cashed-amount check) amount))
        )
        ;; Update check
        (ok (map-set checks
          { check-id: check-id }
          (merge check {
            cashed-amount: new-cashed-amount,
            status: (if (>= new-cashed-amount (get amount check)) STATUS-CASHED STATUS-ACTIVE)
          })
        ))
      )
    )
  )
)

;; Cancel check (sender only)
(define-public (cancel-check (check-id uint))
  (let
    (
      (sender tx-sender)
      (check (unwrap! (map-get? checks { check-id: check-id }) ERR-CHECK-NOT-FOUND))
    )
    (asserts! (is-eq (get sender check) sender) ERR-NOT-AUTHORIZED)
    (asserts! (is-eq (get status check) STATUS-ACTIVE) ERR-CHECK-CASHED)

    (let
      (
        (remaining (- (get amount check) (get cashed-amount check)))
      )
      ;; Return uncashed funds to sender
      (if (> remaining u0)
        (try! (as-contract (stx-transfer? remaining tx-sender sender)))
        true
      )

      ;; Mark as cancelled
      (ok (map-set checks
        { check-id: check-id }
        (merge check { status: STATUS-CANCELLED })
      ))
    )
  )
)

;; Expire check (anyone can call after expiration)
(define-public (expire-check (check-id uint))
  (let
    (
      (check (unwrap! (map-get? checks { check-id: check-id }) ERR-CHECK-NOT-FOUND))
    )
    (asserts! (is-eq (get status check) STATUS-ACTIVE) ERR-CHECK-CASHED)
    (asserts! (>= block-height (get expiration check)) ERR-CHECK-EXPIRED)

    (let
      (
        (remaining (- (get amount check) (get cashed-amount check)))
      )
      ;; Return uncashed funds to sender
      (if (> remaining u0)
        (try! (as-contract (stx-transfer? remaining tx-sender (get sender check))))
        true
      )

      ;; Mark as cancelled
      (ok (map-set checks
        { check-id: check-id }
        (merge check { status: STATUS-CANCELLED })
      ))
    )
  )
)

;; Read-only functions
(define-read-only (get-check (check-id uint))
  (map-get? checks { check-id: check-id })
)

(define-read-only (get-remaining-amount (check-id uint))
  (match (get-check check-id)
    check (ok (- (get amount check) (get cashed-amount check)))
    ERR-CHECK-NOT-FOUND
  )
)

(define-read-only (is-expired (check-id uint))
  (match (get-check check-id)
    check (ok (>= block-height (get expiration check)))
    ERR-CHECK-NOT-FOUND
  )
)

(define-read-only (is-cashable (check-id uint))
  (match (get-check check-id)
    check (ok (and
      (is-eq (get status check) STATUS-ACTIVE)
      (< block-height (get expiration check))
      (< (get cashed-amount check) (get amount check))
    ))
    ERR-CHECK-NOT-FOUND
  )
)

(define-read-only (get-next-check-id)
  (ok (var-get next-check-id))
)
