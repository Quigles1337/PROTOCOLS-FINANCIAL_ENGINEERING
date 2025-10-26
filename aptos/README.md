# Aptos XRPL Financial Primitives

Production-grade implementation of XRPL-inspired financial primitives in Move for the Aptos blockchain.

## Overview

This implementation brings all 10 XRPL financial primitives to Aptos using the Move programming language, leveraging Aptos's resource-oriented architecture for maximum security and efficiency.

## Modules Implemented

All 10 financial primitives are production-ready with comprehensive features:

### 1. **TrustLines** (276 lines)
Bilateral credit lines with payment rippling for multi-hop credit networks.

**Key Features**:
- Bidirectional credit limits
- Real-time balance tracking with positive/negative balances
- Payment rippling for credit propagation
- Deterministic account pairing with ID generation

**Entry Points**:
- `create_trust_line()` - Establish bilateral credit line
- `update_limits()` - Modify credit limits
- `ripple_payment()` - Execute payment through trust line
- `close_trust_line()` - Close line (requires zero balance)

### 2. **PaymentChannels** (297 lines)
Streaming micropayments with off-chain efficiency and on-chain settlement.

**Key Features**:
- Coin-based deposits using Aptos native coins
- Incremental claims by receiver
- Expiration-based sender reclaim
- Partial withdrawal support

**Entry Points**:
- `create_channel()` - Open payment channel
- `add_funds()` - Top up existing channel
- `claim_funds()` - Receiver claims payment
- `close_channel()` - Close channel and return unclaimed funds

### 3. **Escrow** (293 lines)
Time-locked and hash-locked conditional payments (HTLC).

**Key Features**:
- Time-lock with release/cancel windows
- Hash-lock with SHA3-256 preimage verification
- HTLC support for atomic swaps
- Coin-based deposit storage

**Entry Points**:
- `create_time_locked()` - Time-based escrow
- `create_hash_locked()` - HTLC escrow
- `execute_escrow()` - Release with preimage
- `cancel_escrow()` - Cancel after timeout

### 4. **Checks** (280 lines)
Deferred payments like paper checks that recipients can cash later.

**Key Features**:
- Partial cashing support
- Expiration enforcement
- Sender cancellation
- Auto-expiration with refund

**Entry Points**:
- `create_check()` - Issue deferred payment
- `cash_check()` - Claim funds (supports partial)
- `cancel_check()` - Sender cancels
- `expire_check()` - Anyone can expire after timeout

### 5. **DEXOrders** (253 lines)
On-chain orderbook with limit orders and partial fills.

**Key Features**:
- Generic asset type support
- Partial fill capability
- Proportional payment calculation
- Price-time priority (via order ID)

**Entry Points**:
- `place_order<SellAsset, BuyAsset>()` - Create limit order
- `fill_order<SellAsset, BuyAsset>()` - Fill order (partial supported)
- `cancel_order()` - Cancel open/partially filled order

### 6. **DIDManager** (209 lines)
Decentralized identifier management (W3C DID standard).

**Key Features**:
- One DID per address
- Document URI storage
- Revocation support
- Bidirectional DID ↔ Address lookup

**Entry Points**:
- `register_did()` - Create DID
- `update_did()` - Update document URI
- `revoke_did()` - Revoke DID

### 7. **DepositAuthorization** (326 lines)
Multi-tier KYC/AML compliance framework.

**Key Features**:
- Four tier system: Basic, Standard, Premium, Institutional
- Per-asset authorization with TypeInfo
- Time-based expiration
- Amount limits per tier

**Entry Points**:
- `create_authorization<Asset>()` - Grant deposit rights
- `validate_deposit<Asset>()` - Check authorization
- `use_authorization<Asset>()` - Mark as used
- `revoke_authorization<Asset>()` - Revoke access
- `update_tier<Asset>()` - Change KYC tier

### 8. **DepositPreauth** (226 lines)
One-time pre-authorization tokens for controlled deposits.

**Key Features**:
- Single-use tokens
- Per-asset type validation
- Amount limits
- Time-based expiration

**Entry Points**:
- `create_preauth<Asset>()` - Issue one-time token
- `use_preauth<Asset>()` - Consume token
- `revoke_preauth()` - Cancel unused token

### 9. **SignerList** (382 lines)
Weighted multi-signature with proposal-based governance.

**Key Features**:
- Weighted voting system
- Quorum-based execution
- Proposal lifecycle (create → approve → execute)
- Dynamic signer management

**Entry Points**:
- `create_signer_list()` - Initialize multisig
- `add_signer()` / `remove_signer()` - Manage signers
- `create_proposal()` - Submit proposal
- `approve_proposal()` - Vote with weight
- `execute_proposal()` - Execute if quorum met

### 10. **AccountDelete** (329 lines)
Account lifecycle management with 24-hour grace period.

**Key Features**:
- 24-hour grace period (86400 seconds)
- Beneficiary fund recovery
- Cancellable deletion requests
- Coin balance transfer on deletion

**Entry Points**:
- `create_account()` - Register account
- `deposit()` - Add funds to account
- `request_deletion()` - Initiate deletion
- `cancel_deletion()` - Cancel request
- `execute_deletion()` - Finalize after grace period

## Technical Architecture

### Move Design Patterns

All modules follow production-grade Move patterns:

1. **Resource Safety**
   - Proper use of `store`, `copy`, `drop` abilities
   - Coin resources managed securely
   - No resource leaks

2. **Access Control**
   - Signer-based authentication
   - Owner validation
   - Permission checks with proper error codes

3. **Global Storage**
   - Table-based registries at `@xrpl_primitives`
   - Efficient key-value lookups
   - SimpleMap for flexible mappings

4. **Events**
   - Comprehensive event emissions
   - Indexed by relevant IDs
   - Timestamp tracking

5. **View Functions**
   - Read-only queries marked with `#[view]`
   - Validation helpers
   - State inspection

### Error Handling

Each module defines custom error codes:
- `E_NOT_INITIALIZED` - Registry not set up
- `E_UNAUTHORIZED` - Permission denied
- `E_INVALID_*` - Input validation failures
- Module-specific errors for domain logic

### Type System

- **TypeInfo** - Generic asset type handling (DEX, Authorization)
- **Coin<AptosCoin>** - Native coin management
- **Table<K, V>** - Scalable key-value storage
- **SimpleMap<K, V>** - Flexible mappings for complex data

## Project Statistics

```
Total Modules:           10
Total Lines of Code:     2,871 lines
Language:               Move (Aptos)
Framework:              Aptos Framework + Stdlib

Module Breakdown:
  SignerList:           382 lines (Weighted multisig)
  AccountDelete:        329 lines (Lifecycle management)
  DepositAuth:          326 lines (KYC/AML compliance)
  PaymentChannels:      297 lines (Streaming payments)
  Escrow:               293 lines (Time/hash locks)
  Checks:               280 lines (Deferred payments)
  TrustLines:           276 lines (Credit networks)
  DEXOrders:            253 lines (Orderbook)
  DepositPreauth:       226 lines (One-time tokens)
  DIDManager:           209 lines (Decentralized identity)
```

## Getting Started

### Prerequisites

```bash
# Install Aptos CLI
curl -fsSL "https://aptos.dev/scripts/install_cli.py" | python3

# Verify installation
aptos --version
```

### Compile

```bash
cd aptos/
aptos move compile
```

### Test

```bash
aptos move test
```

### Deploy

```bash
# Initialize account
aptos init

# Publish modules
aptos move publish --named-addresses xrpl_primitives=<YOUR_ADDRESS>
```

## Usage Examples

### TrustLines

```move
// Create trust line
trust_lines::create_trust_line(
    &sender,
    counterparty_address,
    1000000,  // limit1
    2000000   // limit2
);

// Ripple payment
trust_lines::ripple_payment(
    &sender,
    receiver_address,
    500000
);
```

### Payment Channels

```move
// Create channel
payment_channels::create_channel(
    &sender,
    receiver_address,
    1000000,    // initial_balance
    expiration  // timestamp
);

// Claim funds
payment_channels::claim_funds(
    &receiver,
    channel_id,
    250000
);
```

### Escrow (HTLC)

```move
// Create hash-locked escrow
escrow::create_hash_locked(
    &sender,
    receiver_address,
    amount,
    release_time,
    cancel_time,
    hash  // SHA3-256 hash
);

// Execute with preimage
escrow::execute_escrow(
    &receiver,
    escrow_id,
    preimage
);
```

### SignerList Multisig

```move
// Create multisig
let list_id = signer_list::create_signer_list(&owner, 7000); // 70% quorum

// Add signers
signer_list::add_signer(&owner, list_id, signer1, 4000); // 40% weight
signer_list::add_signer(&owner, list_id, signer2, 3000); // 30% weight
signer_list::add_signer(&owner, list_id, signer3, 3000); // 30% weight

// Create and approve proposal
let proposal_id = signer_list::create_proposal(&signer1, list_id, b"Transfer 1M");
signer_list::approve_proposal(&signer2, proposal_id); // 40% + 30% = 70% ✓

// Execute
signer_list::execute_proposal(proposal_id);
```

## Security Considerations

1. **Reentrancy**: Move's resource model prevents reentrancy attacks
2. **Integer Overflow**: Move has built-in overflow protection
3. **Access Control**: All sensitive operations require signer authentication
4. **Resource Leaks**: Proper coin extraction and deposit handling
5. **Time Dependencies**: Uses `timestamp::now_seconds()` for consistency

## Contributing

This implementation is part of the PROTOCOLS-FINANCIAL_ENGINEERING project bringing XRPL primitives to all major blockchains.

## License

MIT License - See LICENSE file for details

## Author

**Quigles1337** (adz@alphx.io)

Built with production-grade financial engineering for the Aptos ecosystem.
