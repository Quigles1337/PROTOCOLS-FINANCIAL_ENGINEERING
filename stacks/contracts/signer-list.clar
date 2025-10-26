;; Signer List - Weighted multisig with proposal-based governance
;; Enables DAOs and corporate treasury management

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-LIST-NOT-FOUND (err u2))
(define-constant ERR-PROPOSAL-NOT-FOUND (err u3))
(define-constant ERR-ALREADY-APPROVED (err u4))
(define-constant ERR-NOT-SIGNER (err u5))
(define-constant ERR-QUORUM-NOT-MET (err u6))
(define-constant ERR-ALREADY-EXECUTED (err u7))
(define-constant ERR-SIGNER-EXISTS (err u8))
(define-constant ERR-SIGNER-NOT-FOUND (err u9))

;; Data vars
(define-data-var next-list-id uint u0)
(define-data-var next-proposal-id uint u0)

;; Data maps
(define-map signer-lists
  { list-id: uint }
  {
    owner: principal,
    quorum: uint,
    total-weight: uint,
    created-at: uint
  }
)

(define-map signers
  { list-id: uint, signer: principal }
  { weight: uint }
)

(define-map proposals
  { proposal-id: uint }
  {
    list-id: uint,
    proposer: principal,
    description: (string-ascii 256),
    approvals-weight: uint,
    executed: bool,
    created-at: uint
  }
)

(define-map approvals
  { proposal-id: uint, approver: principal }
  { approved: bool }
)

;; Create signer list
(define-public (create-signer-list (quorum uint))
  (let
    (
      (owner tx-sender)
      (list-id (var-get next-list-id))
    )
    (map-set signer-lists
      { list-id: list-id }
      {
        owner: owner,
        quorum: quorum,
        total-weight: u0,
        created-at: block-height
      }
    )

    (var-set next-list-id (+ list-id u1))
    (ok list-id)
  )
)

;; Add signer
(define-public (add-signer (list-id uint) (new-signer principal) (weight uint))
  (let
    (
      (owner tx-sender)
      (signer-list (unwrap! (map-get? signer-lists { list-id: list-id }) ERR-LIST-NOT-FOUND))
    )
    (asserts! (is-eq (get owner signer-list) owner) ERR-NOT-AUTHORIZED)
    (asserts! (is-none (map-get? signers { list-id: list-id, signer: new-signer })) ERR-SIGNER-EXISTS)

    ;; Add signer
    (map-set signers
      { list-id: list-id, signer: new-signer }
      { weight: weight }
    )

    ;; Update total weight
    (ok (map-set signer-lists
      { list-id: list-id }
      (merge signer-list { total-weight: (+ (get total-weight signer-list) weight) })
    ))
  )
)

;; Remove signer
(define-public (remove-signer (list-id uint) (signer-to-remove principal))
  (let
    (
      (owner tx-sender)
      (signer-list (unwrap! (map-get? signer-lists { list-id: list-id }) ERR-LIST-NOT-FOUND))
      (signer-entry (unwrap! (map-get? signers { list-id: list-id, signer: signer-to-remove }) ERR-SIGNER-NOT-FOUND))
    )
    (asserts! (is-eq (get owner signer-list) owner) ERR-NOT-AUTHORIZED)

    ;; Remove signer
    (map-delete signers { list-id: list-id, signer: signer-to-remove })

    ;; Update total weight
    (ok (map-set signer-lists
      { list-id: list-id }
      (merge signer-list { total-weight: (- (get total-weight signer-list) (get weight signer-entry)) })
    ))
  )
)

;; Create proposal
(define-public (create-proposal (list-id uint) (description (string-ascii 256)))
  (let
    (
      (proposer tx-sender)
      (proposal-id (var-get next-proposal-id))
    )
    (asserts! (is-some (map-get? signers { list-id: list-id, signer: proposer })) ERR-NOT-SIGNER)

    (map-set proposals
      { proposal-id: proposal-id }
      {
        list-id: list-id,
        proposer: proposer,
        description: description,
        approvals-weight: u0,
        executed: false,
        created-at: block-height
      }
    )

    (var-set next-proposal-id (+ proposal-id u1))
    (ok proposal-id)
  )
)

;; Approve proposal
(define-public (approve-proposal (proposal-id uint))
  (let
    (
      (approver tx-sender)
      (proposal (unwrap! (map-get? proposals { proposal-id: proposal-id }) ERR-PROPOSAL-NOT-FOUND))
      (signer-entry (unwrap! (map-get? signers { list-id: (get list-id proposal), signer: approver }) ERR-NOT-SIGNER))
    )
    (asserts! (not (get executed proposal)) ERR-ALREADY-EXECUTED)
    (asserts! (is-none (map-get? approvals { proposal-id: proposal-id, approver: approver })) ERR-ALREADY-APPROVED)

    ;; Record approval
    (map-set approvals
      { proposal-id: proposal-id, approver: approver }
      { approved: true }
    )

    ;; Update approvals weight
    (ok (map-set proposals
      { proposal-id: proposal-id }
      (merge proposal { approvals-weight: (+ (get approvals-weight proposal) (get weight signer-entry)) })
    ))
  )
)

;; Execute proposal (if quorum met)
(define-public (execute-proposal (proposal-id uint))
  (let
    (
      (proposal (unwrap! (map-get? proposals { proposal-id: proposal-id }) ERR-PROPOSAL-NOT-FOUND))
      (signer-list (unwrap! (map-get? signer-lists { list-id: (get list-id proposal) }) ERR-LIST-NOT-FOUND))
    )
    (asserts! (not (get executed proposal)) ERR-ALREADY-EXECUTED)
    (asserts! (>= (get approvals-weight proposal) (get quorum signer-list)) ERR-QUORUM-NOT-MET)

    (ok (map-set proposals
      { proposal-id: proposal-id }
      (merge proposal { executed: true })
    ))
  )
)

;; Read-only functions
(define-read-only (get-signer-list (list-id uint))
  (map-get? signer-lists { list-id: list-id })
)

(define-read-only (get-proposal (proposal-id uint))
  (map-get? proposals { proposal-id: proposal-id })
)

(define-read-only (is-signer (list-id uint) (addr principal))
  (is-some (map-get? signers { list-id: list-id, signer: addr }))
)

(define-read-only (get-signer-weight (list-id uint) (addr principal))
  (match (map-get? signers { list-id: list-id, signer: addr })
    signer (ok (get weight signer))
    ERR-SIGNER-NOT-FOUND
  )
)

(define-read-only (has-approved (proposal-id uint) (addr principal))
  (is-some (map-get? approvals { proposal-id: proposal-id, approver: addr }))
)

(define-read-only (get-next-list-id)
  (ok (var-get next-list-id))
)

(define-read-only (get-next-proposal-id)
  (ok (var-get next-proposal-id))
)
