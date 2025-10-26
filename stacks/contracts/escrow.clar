;; Escrow - Time-locked and hash-locked conditional payments (HTLC)
;; Enables atomic swaps and conditional fund releases

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-ESCROW-NOT-FOUND (err u2))
(define-constant ERR-ALREADY-EXECUTED (err u3))
(define-constant ERR-NOT-RELEASED (err u4))
(define-constant ERR-WRONG-PREIMAGE (err u5))
(define-constant ERR-CANCEL-TOO-EARLY (err u6))
(define-constant ERR-INVALID-AMOUNT (err u7))

;; Constants
(define-constant STATUS-ACTIVE u1)
(define-constant STATUS-EXECUTED u2)
(define-constant STATUS-CANCELLED u3)

;; Data vars
(define-data-var next-escrow-id uint u0)

;; Data maps
(define-map escrows
  { escrow-id: uint }
  {
    sender: principal,
    receiver: principal,
    amount: uint,
    release-time: uint,
    cancel-time: uint,
    condition-hash: (optional (buff 32)),
    status: uint,
    created-at: uint
  }
)

;; Create time-locked escrow
(define-public (create-time-locked (receiver principal) (amount uint) (release-time uint) (cancel-time uint))
  (create-escrow-internal receiver amount release-time cancel-time none)
)

;; Create hash-locked escrow (HTLC)
(define-public (create-hash-locked
    (receiver principal)
    (amount uint)
    (release-time uint)
    (cancel-time uint)
    (condition-hash (buff 32)))
  (create-escrow-internal receiver amount release-time cancel-time (some condition-hash))
)

;; Internal escrow creation
(define-private (create-escrow-internal
    (receiver principal)
    (amount uint)
    (release-time uint)
    (cancel-time uint)
    (condition-hash (optional (buff 32))))
  (let
    (
      (sender tx-sender)
      (escrow-id (var-get next-escrow-id))
    )
    (asserts! (> amount u0) ERR-INVALID-AMOUNT)
    (asserts! (>= release-time block-height) ERR-NOT-RELEASED)
    (asserts! (> cancel-time release-time) ERR-CANCEL-TOO-EARLY)

    ;; Transfer STX to contract
    (try! (stx-transfer? amount sender (as-contract tx-sender)))

    ;; Create escrow
    (map-set escrows
      { escrow-id: escrow-id }
      {
        sender: sender,
        receiver: receiver,
        amount: amount,
        release-time: release-time,
        cancel-time: cancel-time,
        condition-hash: condition-hash,
        status: STATUS-ACTIVE,
        created-at: block-height
      }
    )

    (var-set next-escrow-id (+ escrow-id u1))
    (ok escrow-id)
  )
)

;; Execute escrow (with optional preimage for hash-locked)
(define-public (execute-escrow (escrow-id uint) (preimage (optional (buff 32))))
  (let
    (
      (receiver tx-sender)
      (escrow (unwrap! (map-get? escrows { escrow-id: escrow-id }) ERR-ESCROW-NOT-FOUND))
    )
    (asserts! (is-eq (get receiver escrow) receiver) ERR-NOT-AUTHORIZED)
    (asserts! (is-eq (get status escrow) STATUS-ACTIVE) ERR-ALREADY-EXECUTED)
    (asserts! (>= block-height (get release-time escrow)) ERR-NOT-RELEASED)
    (asserts! (< block-height (get cancel-time escrow)) ERR-CANCEL-TOO-EARLY)

    ;; Verify hash condition if present
    (match (get condition-hash escrow)
      hash (match preimage
        pre (asserts! (is-eq (sha256 pre) hash) ERR-WRONG-PREIMAGE)
        ERR-WRONG-PREIMAGE
      )
      true
    )

    ;; Transfer funds to receiver
    (try! (as-contract (stx-transfer? (get amount escrow) tx-sender receiver)))

    ;; Mark as executed
    (ok (map-set escrows
      { escrow-id: escrow-id }
      (merge escrow { status: STATUS-EXECUTED })
    ))
  )
)

;; Cancel escrow (sender only, after cancel time)
(define-public (cancel-escrow (escrow-id uint))
  (let
    (
      (sender tx-sender)
      (escrow (unwrap! (map-get? escrows { escrow-id: escrow-id }) ERR-ESCROW-NOT-FOUND))
    )
    (asserts! (is-eq (get sender escrow) sender) ERR-NOT-AUTHORIZED)
    (asserts! (is-eq (get status escrow) STATUS-ACTIVE) ERR-ALREADY-EXECUTED)
    (asserts! (>= block-height (get cancel-time escrow)) ERR-CANCEL-TOO-EARLY)

    ;; Return funds to sender
    (try! (as-contract (stx-transfer? (get amount escrow) tx-sender sender)))

    ;; Mark as cancelled
    (ok (map-set escrows
      { escrow-id: escrow-id }
      (merge escrow { status: STATUS-CANCELLED })
    ))
  )
)

;; Read-only functions
(define-read-only (get-escrow (escrow-id uint))
  (map-get? escrows { escrow-id: escrow-id })
)

(define-read-only (is-executable (escrow-id uint))
  (match (get-escrow escrow-id)
    escrow (ok (and
      (is-eq (get status escrow) STATUS-ACTIVE)
      (>= block-height (get release-time escrow))
      (< block-height (get cancel-time escrow))
    ))
    ERR-ESCROW-NOT-FOUND
  )
)

(define-read-only (is-cancellable (escrow-id uint))
  (match (get-escrow escrow-id)
    escrow (ok (and
      (is-eq (get status escrow) STATUS-ACTIVE)
      (>= block-height (get cancel-time escrow))
    ))
    ERR-ESCROW-NOT-FOUND
  )
)

(define-read-only (has-condition (escrow-id uint))
  (match (get-escrow escrow-id)
    escrow (ok (is-some (get condition-hash escrow)))
    ERR-ESCROW-NOT-FOUND
  )
)

(define-read-only (get-next-escrow-id)
  (ok (var-get next-escrow-id))
)
