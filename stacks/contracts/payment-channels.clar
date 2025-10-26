;; Payment Channels - Streaming micropayments with off-chain efficiency
;; Enables real-time payments with on-chain settlement

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-CHANNEL-NOT-FOUND (err u2))
(define-constant ERR-CHANNEL-CLOSED (err u3))
(define-constant ERR-INVALID-AMOUNT (err u4))
(define-constant ERR-INSUFFICIENT-BALANCE (err u5))
(define-constant ERR-EXPIRATION-NOT-REACHED (err u6))
(define-constant ERR-CLAIM-TOO-HIGH (err u7))
(define-constant ERR-SAME-ACCOUNT (err u8))

;; Constants
(define-constant STATUS-OPEN u1)
(define-constant STATUS-CLOSED u2)

;; Data vars
(define-data-var next-channel-id uint u0)

;; Data maps
(define-map channels
  { channel-id: uint }
  {
    sender: principal,
    receiver: principal,
    balance: uint,
    total-claimed: uint,
    expiration: uint,
    status: uint,
    created-at: uint
  }
)

;; Create payment channel
(define-public (create-channel (receiver principal) (initial-balance uint) (expiration uint))
  (let
    (
      (sender tx-sender)
      (channel-id (var-get next-channel-id))
    )
    (asserts! (not (is-eq sender receiver)) ERR-SAME-ACCOUNT)
    (asserts! (> initial-balance u0) ERR-INVALID-AMOUNT)
    (asserts! (> expiration block-height) ERR-EXPIRATION-NOT-REACHED)

    ;; Transfer STX to contract
    (try! (stx-transfer? initial-balance sender (as-contract tx-sender)))

    ;; Create channel
    (map-set channels
      { channel-id: channel-id }
      {
        sender: sender,
        receiver: receiver,
        balance: initial-balance,
        total-claimed: u0,
        expiration: expiration,
        status: STATUS-OPEN,
        created-at: block-height
      }
    )

    (var-set next-channel-id (+ channel-id u1))
    (ok channel-id)
  )
)

;; Add funds to existing channel
(define-public (add-funds (channel-id uint) (amount uint))
  (let
    (
      (sender tx-sender)
      (channel (unwrap! (map-get? channels { channel-id: channel-id }) ERR-CHANNEL-NOT-FOUND))
    )
    (asserts! (is-eq (get sender channel) sender) ERR-NOT-AUTHORIZED)
    (asserts! (is-eq (get status channel) STATUS-OPEN) ERR-CHANNEL-CLOSED)
    (asserts! (> amount u0) ERR-INVALID-AMOUNT)

    ;; Transfer STX to contract
    (try! (stx-transfer? amount sender (as-contract tx-sender)))

    ;; Update balance
    (ok (map-set channels
      { channel-id: channel-id }
      (merge channel { balance: (+ (get balance channel) amount) })
    ))
  )
)

;; Claim funds from channel (receiver only)
(define-public (claim-funds (channel-id uint) (amount uint))
  (let
    (
      (receiver tx-sender)
      (channel (unwrap! (map-get? channels { channel-id: channel-id }) ERR-CHANNEL-NOT-FOUND))
    )
    (asserts! (is-eq (get receiver channel) receiver) ERR-NOT-AUTHORIZED)
    (asserts! (is-eq (get status channel) STATUS-OPEN) ERR-CHANNEL-CLOSED)
    (asserts! (> amount u0) ERR-INVALID-AMOUNT)

    (let
      (
        (available (- (get balance channel) (get total-claimed channel)))
      )
      (asserts! (<= amount available) ERR-CLAIM-TOO-HIGH)

      ;; Transfer STX to receiver
      (try! (as-contract (stx-transfer? amount tx-sender receiver)))

      ;; Update total claimed
      (ok (map-set channels
        { channel-id: channel-id }
        (merge channel { total-claimed: (+ (get total-claimed channel) amount) })
      ))
    )
  )
)

;; Close channel (sender after expiration, receiver anytime)
(define-public (close-channel (channel-id uint))
  (let
    (
      (caller tx-sender)
      (channel (unwrap! (map-get? channels { channel-id: channel-id }) ERR-CHANNEL-NOT-FOUND))
    )
    (asserts! (is-eq (get status channel) STATUS-OPEN) ERR-CHANNEL-CLOSED)

    ;; Authorization check
    (if (is-eq (get sender channel) caller)
      ;; Sender can close after expiration
      (asserts! (>= block-height (get expiration channel)) ERR-EXPIRATION-NOT-REACHED)
      ;; Receiver can close anytime
      (asserts! (is-eq (get receiver channel) caller) ERR-NOT-AUTHORIZED)
    )

    (let
      (
        (remaining (- (get balance channel) (get total-claimed channel)))
      )
      ;; Return unclaimed funds to sender if any
      (if (> remaining u0)
        (try! (as-contract (stx-transfer? remaining tx-sender (get sender channel))))
        true
      )

      ;; Mark as closed
      (ok (map-set channels
        { channel-id: channel-id }
        (merge channel { status: STATUS-CLOSED })
      ))
    )
  )
)

;; Read-only functions
(define-read-only (get-channel (channel-id uint))
  (map-get? channels { channel-id: channel-id })
)

(define-read-only (get-available-balance (channel-id uint))
  (match (get-channel channel-id)
    channel (ok (- (get balance channel) (get total-claimed channel)))
    ERR-CHANNEL-NOT-FOUND
  )
)

(define-read-only (is-expired (channel-id uint))
  (match (get-channel channel-id)
    channel (ok (>= block-height (get expiration channel)))
    ERR-CHANNEL-NOT-FOUND
  )
)

(define-read-only (get-next-channel-id)
  (ok (var-get next-channel-id))
)
