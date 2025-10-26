# Stacks XRPL Financial Primitives

Production-grade implementation of XRPL-inspired financial primitives in Clarity for the Stacks blockchain.

## Overview

This implementation brings all 10 XRPL financial primitives to Stacks using Clarity, leveraging Stacks's decidable smart contract language for maximum security and predictability.

## Contracts Implemented

All 10 financial primitives are production-ready with comprehensive features:

### 1. **TrustLines** (190 lines)
Bilateral credit lines with payment rippling for multi-hop credit networks.

**Key Features**:
- Bidirectional credit limits with positive/negative balance tracking
- Deterministic address ordering for consistent map keys
- Payment rippling for credit propagation
- Zero-balance requirement for closure

**Public Functions**:
- `create-trust-line` - Establish bilateral credit line
- `update-limit` - Modify credit limits
- `ripple-payment` - Execute payment through trust line
- `close-trust-line` - Close line (requires zero balance)

### 2. **Payment Channels** (176 lines)
Streaming micropayments with off-chain efficiency and on-chain settlement.

**Key Features**:
- STX-based deposits using native currency
- Incremental claims by receiver
- Expiration-based sender reclaim
- Proper STX transfer handling with `as-contract`

**Public Functions**:
- `create-channel` - Open payment channel
- `add-funds` - Top up existing channel
- `claim-funds` - Receiver claims payment
- `close-channel` - Close channel and return unclaimed funds

### 3. **Escrow** (179 lines)
Time-locked and hash-locked conditional payments (HTLC).

**Key Features**:
- Time-lock with release/cancel windows
- Hash-lock with SHA-256 preimage verification
- HTLC support for atomic swaps
- Separate time-locked and hash-locked creation functions

**Public Functions**:
- `create-time-locked` - Time-based escrow
- `create-hash-locked` - HTLC escrow
- `execute-escrow` - Release with optional preimage
- `cancel-escrow` - Cancel after timeout

### 4. **Checks** (193 lines)
Deferred payments like paper checks that recipients can cash later.

**Key Features**:
- Partial cashing support
- Expiration enforcement
- Sender cancellation
- Public expiration function (anyone can trigger)

**Public Functions**:
- `create-check` - Issue deferred payment
- `cash-check` - Claim funds (supports partial)
- `cancel-check` - Sender cancels
- `expire-check` - Anyone can expire after timeout

### 5. **DEXOrders** (157 lines)
On-chain orderbook with limit orders and partial fills.

**Key Features**:
- String-based asset identifiers
- Partial fill capability
- Proportional payment calculation
- Order lifecycle (Open → PartiallyFilled → Filled/Cancelled)

**Public Functions**:
- `place-order` - Create limit order
- `fill-order` - Fill order (partial supported)
- `cancel-order` - Cancel open/partially filled order

### 6. **DIDManager** (126 lines)
Decentralized identifier management (W3C DID standard).

**Key Features**:
- One DID per address
- Document URI storage
- Revocation support
- Bidirectional DID ↔ Principal lookup

**Public Functions**:
- `register-did` - Create DID
- `update-did` - Update document URI
- `revoke-did` - Revoke DID

### 7. **DepositAuthorization** (161 lines)
Multi-tier KYC/AML compliance framework.

**Key Features**:
- Four tier system: Basic, Standard, Premium, Institutional
- Per-asset authorization
- Time-based expiration
- Amount limits per tier

**Public Functions**:
- `create-authorization` - Grant deposit rights
- `use-authorization` - Mark as used
- `revoke-authorization` - Revoke access
- `update-tier` - Change KYC tier

### 8. **DepositPreauth** (121 lines)
One-time pre-authorization tokens for controlled deposits.

**Key Features**:
- Single-use tokens
- Per-asset validation
- Amount limits
- Time-based expiration

**Public Functions**:
- `create-preauth` - Issue one-time token
- `use-preauth` - Consume token
- `revoke-preauth` - Cancel unused token

### 9. **SignerList** (217 lines)
Weighted multi-signature with proposal-based governance.

**Key Features**:
- Weighted voting system
- Quorum-based execution
- Proposal lifecycle (create → approve → execute)
- Dynamic signer management

**Public Functions**:
- `create-signer-list` - Initialize multisig
- `add-signer` / `remove-signer` - Manage signers
- `create-proposal` - Submit proposal
- `approve-proposal` - Vote with weight
- `execute-proposal` - Execute if quorum met

### 10. **AccountDelete** (198 lines)
Account lifecycle management with ~24-hour grace period.

**Key Features**:
- 144 block grace period (~24 hours with 10 min blocks)
- Beneficiary fund recovery
- Cancellable deletion requests
- STX balance transfer on deletion

**Public Functions**:
- `create-account` - Register account
- `deposit` - Add funds to account
- `request-deletion` - Initiate deletion
- `cancel-deletion` - Cancel request
- `execute-deletion` - Finalize after grace period

## Technical Architecture

### Clarity Design Patterns

All contracts follow production-grade Clarity patterns:

1. **Decidability**
   - No recursion or unbounded loops
   - All execution paths terminate
   - Predictable gas costs

2. **Error Handling**
   - Response types: `(ok ...)` and `(err ...)`
   - Consistent error codes across contracts
   - Descriptive error constants

3. **Storage Layer**
   - Maps for key-value storage
   - Data vars for counters and state
   - Principal-based access control

4. **Access Control**
   - `tx-sender` for authentication
   - Owner validation checks
   - Permission assertions with `asserts!`

5. **Read-Only Functions**
   - `define-read-only` for view functions
   - State inspection without modification
   - Validation helpers

### STX Handling

Proper native currency management:
- `stx-transfer?` for deposits
- `as-contract` for contract-controlled transfers
- Balance tracking in maps

### Error Codes

Each contract defines custom error codes:
- `ERR-NOT-AUTHORIZED` - Permission denied
- `ERR-*-NOT-FOUND` - Entity lookup failures
- `ERR-INVALID-*` - Input validation failures
- Domain-specific errors per contract

## Project Statistics

```
Total Contracts:         10
Total Lines of Code:     1,718 lines
Language:               Clarity
Blockchain:             Stacks

Contract Breakdown:
  SignerList:           217 lines (Weighted multisig)
  AccountDelete:        198 lines (Lifecycle management)
  Checks:               193 lines (Deferred payments)
  TrustLines:           190 lines (Credit networks)
  Escrow:               179 lines (Time/hash locks)
  PaymentChannels:      176 lines (Streaming payments)
  DepositAuth:          161 lines (KYC/AML compliance)
  DEXOrders:            157 lines (Orderbook)
  DIDManager:           126 lines (Decentralized identity)
  DepositPreauth:       121 lines (One-time tokens)
```

## Getting Started

### Prerequisites

```bash
# Install Clarinet (Clarity development tool)
curl -L https://github.com/hirosystems/clarinet/releases/latest/download/clarinet-linux-x64.tar.gz | tar xz
sudo mv clarinet /usr/local/bin/

# Verify installation
clarinet --version
```

### Test Contracts

```bash
cd stacks/
clarinet check
```

### Deploy to Testnet

```bash
# Configure testnet
clarinet integrate

# Deploy contracts
clarinet deploy --testnet
```

## Usage Examples

### TrustLines

```clarity
;; Create trust line
(contract-call? .trust-lines create-trust-line
  'SP2...COUNTERPARTY
  u1000000  ;; limit1
  u2000000  ;; limit2
)

;; Ripple payment
(contract-call? .trust-lines ripple-payment
  'SP2...RECEIVER
  u500000
)
```

### Payment Channels

```clarity
;; Create channel
(contract-call? .payment-channels create-channel
  'SP2...RECEIVER
  u1000000    ;; initial-balance
  u1000       ;; expiration (block height)
)

;; Claim funds
(contract-call? .payment-channels claim-funds
  u0          ;; channel-id
  u250000     ;; amount
)
```

### Escrow (HTLC)

```clarity
;; Create hash-locked escrow
(contract-call? .escrow create-hash-locked
  'SP2...RECEIVER
  u1000000       ;; amount
  u500           ;; release-time
  u1000          ;; cancel-time
  0x1234...abcd  ;; condition-hash (32 bytes)
)

;; Execute with preimage
(contract-call? .escrow execute-escrow
  u0                    ;; escrow-id
  (some 0x5678...cdef)  ;; preimage
)
```

### SignerList Multisig

```clarity
;; Create multisig
(contract-call? .signer-list create-signer-list u7000) ;; 70% quorum

;; Add signers
(contract-call? .signer-list add-signer u0 'SP2...SIGNER1 u4000) ;; 40%
(contract-call? .signer-list add-signer u0 'SP2...SIGNER2 u3000) ;; 30%
(contract-call? .signer-list add-signer u0 'SP2...SIGNER3 u3000) ;; 30%

;; Create and approve proposal
(contract-call? .signer-list create-proposal u0 "Transfer 1M STX")
(contract-call? .signer-list approve-proposal u0)

;; Execute (if quorum met)
(contract-call? .signer-list execute-proposal u0)
```

## Security Considerations

1. **Decidability**: Clarity's decidability prevents reentrancy and infinite loops
2. **Post-Conditions**: Can add post-conditions for additional security
3. **No Integer Overflow**: Clarity has built-in overflow protection
4. **Access Control**: All sensitive operations require `tx-sender` verification
5. **Block Height**: Used for time-based logic (more reliable than timestamps)

## Testing

Clarity contracts can be tested with Clarinet:

```bash
# Run all tests
clarinet test

# Check contract syntax
clarinet check

# Interactive console
clarinet console
```

## Contributing

This implementation is part of the PROTOCOLS-FINANCIAL_ENGINEERING project bringing XRPL primitives to all major blockchains.

## License

MIT License - See LICENSE file for details

## Author

**Quigles1337** (adz@alphx.io)

Built with production-grade financial engineering for the Stacks ecosystem.
