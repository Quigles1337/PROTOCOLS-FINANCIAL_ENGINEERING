;; DID Manager - Decentralized Identifier management (W3C DID standard)
;; Enables self-sovereign identity on Stacks

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u1))
(define-constant ERR-DID-NOT-FOUND (err u2))
(define-constant ERR-DID-EXISTS (err u3))
(define-constant ERR-DID-REVOKED (err u4))
(define-constant ERR-INVALID-DID (err u5))

;; Data maps
(define-map dids
  { owner: principal }
  {
    did: (string-ascii 128),
    document-uri: (string-ascii 256),
    active: bool,
    created-at: uint,
    updated-at: uint
  }
)

(define-map did-to-owner
  { did: (string-ascii 128) }
  { owner: principal }
)

;; Register DID
(define-public (register-did (did (string-ascii 128)) (document-uri (string-ascii 256)))
  (let
    (
      (owner tx-sender)
    )
    (asserts! (> (len did) u0) ERR-INVALID-DID)
    (asserts! (is-none (map-get? dids { owner: owner })) ERR-DID-EXISTS)
    (asserts! (is-none (map-get? did-to-owner { did: did })) ERR-DID-EXISTS)

    ;; Create DID document
    (map-set dids
      { owner: owner }
      {
        did: did,
        document-uri: document-uri,
        active: true,
        created-at: block-height,
        updated-at: block-height
      }
    )

    ;; Create reverse mapping
    (map-set did-to-owner
      { did: did }
      { owner: owner }
    )

    (ok true)
  )
)

;; Update DID document
(define-public (update-did (new-document-uri (string-ascii 256)))
  (let
    (
      (owner tx-sender)
      (did-doc (unwrap! (map-get? dids { owner: owner }) ERR-DID-NOT-FOUND))
    )
    (asserts! (get active did-doc) ERR-DID-REVOKED)

    (ok (map-set dids
      { owner: owner }
      (merge did-doc {
        document-uri: new-document-uri,
        updated-at: block-height
      })
    ))
  )
)

;; Revoke DID
(define-public (revoke-did)
  (let
    (
      (owner tx-sender)
      (did-doc (unwrap! (map-get? dids { owner: owner }) ERR-DID-NOT-FOUND))
    )
    (asserts! (get active did-doc) ERR-DID-REVOKED)

    (ok (map-set dids
      { owner: owner }
      (merge did-doc {
        active: false,
        updated-at: block-height
      })
    ))
  )
)

;; Read-only functions
(define-read-only (get-did-by-owner (owner principal))
  (map-get? dids { owner: owner })
)

(define-read-only (get-owner-by-did (did (string-ascii 128)))
  (match (map-get? did-to-owner { did: did })
    mapping (ok (get owner mapping))
    ERR-DID-NOT-FOUND
  )
)

(define-read-only (has-did (owner principal))
  (is-some (map-get? dids { owner: owner }))
)

(define-read-only (is-did-active (owner principal))
  (match (get-did-by-owner owner)
    did-doc (ok (get active did-doc))
    ERR-DID-NOT-FOUND
  )
)

(define-read-only (get-document-uri (owner principal))
  (match (get-did-by-owner owner)
    did-doc (ok (get document-uri did-doc))
    ERR-DID-NOT-FOUND
  )
)
