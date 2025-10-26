;; Account Delete - Account lifecycle management with grace period
;; Enables safe account deletion with beneficiary fund recovery

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-REQUEST-NOT-FOUND (err u2))
(define-constant ERR-GRACE-PERIOD-NOT-ENDED (err u3))
(define-constant ERR-ALREADY-EXECUTED (err u4))
(define-constant ERR-REQUEST-EXISTS (err u5))
(define-constant ERR-ACCOUNT-NOT-FOUND (err u6))

;; Constants
(define-constant STATUS-ACTIVE u1)
(define-constant STATUS-PENDING-DELETION u2)
(define-constant STATUS-DELETED u3)
(define-constant GRACE-PERIOD u144) ;; ~24 hours in blocks (10 min blocks)

;; Data maps
(define-map accounts
  { owner: principal }
  {
    status: uint,
    balance: uint,
    created-at: uint
  }
)

(define-map deletion-requests
  { owner: principal }
  {
    beneficiary: principal,
    grace-period-end: uint,
    executed: bool,
    created-at: uint
  }
)

;; Create account
(define-public (create-account)
  (let
    (
      (owner tx-sender)
    )
    (ok (map-set accounts
      { owner: owner }
      {
        status: STATUS-ACTIVE,
        balance: u0,
        created-at: block-height
      }
    ))
  )
)

;; Deposit to account
(define-public (deposit (amount uint))
  (let
    (
      (owner tx-sender)
      (account (unwrap! (map-get? accounts { owner: owner }) ERR-ACCOUNT-NOT-FOUND))
    )
    ;; Transfer STX to contract
    (try! (stx-transfer? amount owner (as-contract tx-sender)))

    ;; Update balance
    (ok (map-set accounts
      { owner: owner }
      (merge account { balance: (+ (get balance account) amount) })
    ))
  )
)

;; Request account deletion
(define-public (request-deletion (beneficiary principal))
  (let
    (
      (owner tx-sender)
      (account (unwrap! (map-get? accounts { owner: owner }) ERR-ACCOUNT-NOT-FOUND))
    )
    (asserts! (is-none (map-get? deletion-requests { owner: owner })) ERR-REQUEST-EXISTS)

    (let
      (
        (grace-period-end (+ block-height GRACE-PERIOD))
      )
      ;; Create deletion request
      (map-set deletion-requests
        { owner: owner }
        {
          beneficiary: beneficiary,
          grace-period-end: grace-period-end,
          executed: false,
          created-at: block-height
        }
      )

      ;; Update account status
      (ok (map-set accounts
        { owner: owner }
        (merge account { status: STATUS-PENDING-DELETION })
      ))
    )
  )
)

;; Cancel deletion request
(define-public (cancel-deletion)
  (let
    (
      (owner tx-sender)
      (account (unwrap! (map-get? accounts { owner: owner }) ERR-ACCOUNT-NOT-FOUND))
      (deletion-request (unwrap! (map-get? deletion-requests { owner: owner }) ERR-REQUEST-NOT-FOUND))
    )
    (asserts! (not (get executed deletion-request)) ERR-ALREADY-EXECUTED)

    ;; Remove deletion request
    (map-delete deletion-requests { owner: owner })

    ;; Update account status
    (ok (map-set accounts
      { owner: owner }
      (merge account { status: STATUS-ACTIVE })
    ))
  )
)

;; Execute account deletion (after grace period)
(define-public (execute-deletion (account-to-delete principal))
  (let
    (
      (caller tx-sender)
      (account (unwrap! (map-get? accounts { owner: account-to-delete }) ERR-ACCOUNT-NOT-FOUND))
      (deletion-request (unwrap! (map-get? deletion-requests { owner: account-to-delete }) ERR-REQUEST-NOT-FOUND))
    )
    (asserts! (or
      (is-eq caller account-to-delete)
      (is-eq caller (get beneficiary deletion-request))
    ) ERR-NOT-AUTHORIZED)
    (asserts! (not (get executed deletion-request)) ERR-ALREADY-EXECUTED)
    (asserts! (>= block-height (get grace-period-end deletion-request)) ERR-GRACE-PERIOD-NOT-ENDED)

    ;; Transfer balance to beneficiary if any
    (let
      (
        (balance (get balance account))
      )
      (if (> balance u0)
        (try! (as-contract (stx-transfer? balance tx-sender (get beneficiary deletion-request))))
        true
      )

      ;; Mark as executed
      (map-set deletion-requests
        { owner: account-to-delete }
        (merge deletion-request { executed: true })
      )

      ;; Update account status
      (ok (map-set accounts
        { owner: account-to-delete }
        (merge account { status: STATUS-DELETED, balance: u0 })
      ))
    )
  )
)

;; Read-only functions
(define-read-only (get-account (owner principal))
  (map-get? accounts { owner: owner })
)

(define-read-only (get-deletion-request (owner principal))
  (map-get? deletion-requests { owner: owner })
)

(define-read-only (get-balance (owner principal))
  (match (get-account owner)
    account (ok (get balance account))
    (ok u0)
  )
)

(define-read-only (is-pending-deletion (owner principal))
  (match (get-account owner)
    account (ok (is-eq (get status account) STATUS-PENDING-DELETION))
    (ok false)
  )
)

(define-read-only (can-execute-deletion (owner principal))
  (match (get-deletion-request owner)
    deletion-request (ok (and
      (not (get executed deletion-request))
      (>= block-height (get grace-period-end deletion-request))
    ))
    (ok false)
  )
)
