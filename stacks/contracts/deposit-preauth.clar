;; Deposit Preauth - One-time pre-authorization tokens
;; Single-use deposit permissions with amount limits

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-PREAUTH-NOT-FOUND (err u2))
(define-constant ERR-ALREADY-USED (err u3))
(define-constant ERR-EXPIRED (err u4))
(define-constant ERR-AMOUNT-EXCEEDS-MAX (err u5))

;; Data vars
(define-data-var next-preauth-id uint u0)

;; Data maps
(define-map preauths
  { preauth-id: uint }
  {
    authorizer: principal,
    authorized: principal,
    asset: (string-ascii 10),
    max-amount: uint,
    expiration: uint,
    used: bool,
    created-at: uint
  }
)

;; Create preauthorization
(define-public (create-preauth
    (authorized principal)
    (asset (string-ascii 10))
    (max-amount uint)
    (expiration uint))
  (let
    (
      (authorizer tx-sender)
      (preauth-id (var-get next-preauth-id))
    )
    (asserts! (> expiration block-height) ERR-EXPIRED)

    (map-set preauths
      { preauth-id: preauth-id }
      {
        authorizer: authorizer,
        authorized: authorized,
        asset: asset,
        max-amount: max-amount,
        expiration: expiration,
        used: false,
        created-at: block-height
      }
    )

    (var-set next-preauth-id (+ preauth-id u1))
    (ok preauth-id)
  )
)

;; Use preauthorization (single use only)
(define-public (use-preauth (preauth-id uint) (amount uint))
  (let
    (
      (authorized tx-sender)
      (preauth (unwrap! (map-get? preauths { preauth-id: preauth-id }) ERR-PREAUTH-NOT-FOUND))
    )
    (asserts! (is-eq (get authorized preauth) authorized) ERR-NOT-AUTHORIZED)
    (asserts! (not (get used preauth)) ERR-ALREADY-USED)
    (asserts! (< block-height (get expiration preauth)) ERR-EXPIRED)
    (asserts! (<= amount (get max-amount preauth)) ERR-AMOUNT-EXCEEDS-MAX)

    ;; Mark as used (single-use token)
    (ok (map-set preauths
      { preauth-id: preauth-id }
      (merge preauth { used: true })
    ))
  )
)

;; Revoke preauthorization
(define-public (revoke-preauth (preauth-id uint))
  (let
    (
      (authorizer tx-sender)
      (preauth (unwrap! (map-get? preauths { preauth-id: preauth-id }) ERR-PREAUTH-NOT-FOUND))
    )
    (asserts! (is-eq (get authorizer preauth) authorizer) ERR-NOT-AUTHORIZED)
    (asserts! (not (get used preauth)) ERR-ALREADY-USED)

    ;; Mark as used to prevent future use
    (ok (map-set preauths
      { preauth-id: preauth-id }
      (merge preauth { used: true })
    ))
  )
)

;; Read-only functions
(define-read-only (get-preauth (preauth-id uint))
  (map-get? preauths { preauth-id: preauth-id })
)

(define-read-only (is-valid (preauth-id uint))
  (match (get-preauth preauth-id)
    preauth (ok (and
      (not (get used preauth))
      (< block-height (get expiration preauth))
    ))
    (ok false)
  )
)

(define-read-only (is-expired (preauth-id uint))
  (match (get-preauth preauth-id)
    preauth (ok (>= block-height (get expiration preauth)))
    (ok false)
  )
)

(define-read-only (get-next-preauth-id)
  (ok (var-get next-preauth-id))
)
