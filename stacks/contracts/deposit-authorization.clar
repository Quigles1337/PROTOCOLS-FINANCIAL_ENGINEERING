;; Deposit Authorization - Multi-tier KYC/AML compliance
;; Enables regulatory compliance with tiered access control

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-AUTH-NOT-FOUND (err u2))
(define-constant ERR-AUTH-EXPIRED (err u3))
(define-constant ERR-AUTH-REVOKED (err u4))
(define-constant ERR-INVALID-TIER (err u5))
(define-constant ERR-AMOUNT-EXCEEDS-LIMIT (err u6))

;; Authorization tiers (KYC/AML levels)
(define-constant TIER-BASIC u1)
(define-constant TIER-STANDARD u2)
(define-constant TIER-PREMIUM u3)
(define-constant TIER-INSTITUTIONAL u4)

;; Data maps
(define-map authorizations
  { authorizer: principal, authorized: principal, asset: (string-ascii 10) }
  {
    max-amount: uint,
    expiration: uint,
    tier: uint,
    active: bool,
    created-at: uint
  }
)

;; Create authorization
(define-public (create-authorization
    (authorized principal)
    (asset (string-ascii 10))
    (max-amount uint)
    (expiration uint)
    (tier uint))
  (let
    (
      (authorizer tx-sender)
    )
    (asserts! (is-valid-tier tier) ERR-INVALID-TIER)
    (asserts! (> expiration block-height) ERR-AUTH-EXPIRED)

    (ok (map-set authorizations
      { authorizer: authorizer, authorized: authorized, asset: asset }
      {
        max-amount: max-amount,
        expiration: expiration,
        tier: tier,
        active: true,
        created-at: block-height
      }
    ))
  )
)

;; Validate deposit (read-only check)
(define-read-only (validate-deposit
    (authorizer principal)
    (authorized principal)
    (asset (string-ascii 10))
    (amount uint))
  (match (map-get? authorizations { authorizer: authorizer, authorized: authorized, asset: asset })
    auth (ok (and
      (get active auth)
      (< block-height (get expiration auth))
      (<= amount (get max-amount auth))
    ))
    (ok false)
  )
)

;; Use authorization
(define-public (use-authorization
    (authorizer principal)
    (asset (string-ascii 10))
    (amount uint))
  (let
    (
      (authorized tx-sender)
      (auth (unwrap! (map-get? authorizations { authorizer: authorizer, authorized: authorized, asset: asset }) ERR-AUTH-NOT-FOUND))
    )
    (asserts! (get active auth) ERR-AUTH-REVOKED)
    (asserts! (< block-height (get expiration auth)) ERR-AUTH-EXPIRED)
    (asserts! (<= amount (get max-amount auth)) ERR-AMOUNT-EXCEEDS-LIMIT)

    (ok true)
  )
)

;; Revoke authorization
(define-public (revoke-authorization (authorized principal) (asset (string-ascii 10)))
  (let
    (
      (authorizer tx-sender)
      (auth (unwrap! (map-get? authorizations { authorizer: authorizer, authorized: authorized, asset: asset }) ERR-AUTH-NOT-FOUND))
    )
    (ok (map-set authorizations
      { authorizer: authorizer, authorized: authorized, asset: asset }
      (merge auth { active: false })
    ))
  )
)

;; Update tier
(define-public (update-tier (authorized principal) (asset (string-ascii 10)) (new-tier uint))
  (let
    (
      (authorizer tx-sender)
      (auth (unwrap! (map-get? authorizations { authorizer: authorizer, authorized: authorized, asset: asset }) ERR-AUTH-NOT-FOUND))
    )
    (asserts! (is-valid-tier new-tier) ERR-INVALID-TIER)

    (ok (map-set authorizations
      { authorizer: authorizer, authorized: authorized, asset: asset }
      (merge auth { tier: new-tier })
    ))
  )
)

;; Helper functions
(define-private (is-valid-tier (tier uint))
  (or
    (is-eq tier TIER-BASIC)
    (or
      (is-eq tier TIER-STANDARD)
      (or
        (is-eq tier TIER-PREMIUM)
        (is-eq tier TIER-INSTITUTIONAL)
      )
    )
  )
)

;; Read-only functions
(define-read-only (get-authorization
    (authorizer principal)
    (authorized principal)
    (asset (string-ascii 10)))
  (map-get? authorizations { authorizer: authorizer, authorized: authorized, asset: asset })
)

(define-read-only (is-authorized
    (authorizer principal)
    (authorized principal)
    (asset (string-ascii 10)))
  (match (get-authorization authorizer authorized asset)
    auth (ok (and (get active auth) (< block-height (get expiration auth))))
    (ok false)
  )
)

(define-read-only (get-tier
    (authorizer principal)
    (authorized principal)
    (asset (string-ascii 10)))
  (match (get-authorization authorizer authorized asset)
    auth (ok (get tier auth))
    ERR-AUTH-NOT-FOUND
  )
)
