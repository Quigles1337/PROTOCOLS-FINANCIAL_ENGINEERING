# XRPL Financial Primitives for Sui

**Production-grade implementation of all 10 XRPL-inspired financial primitives in Sui Move**

## Overview

This implementation brings XRPL's battle-tested financial primitives to the Sui blockchain, leveraging Sui's unique object-centric architecture and Move programming language for maximum safety and performance.

## Key Features

- ✅ **Object-Centric Architecture**: Leverages Sui's owned and shared objects
- ✅ **Move Language Safety**: Resource-oriented programming with no dangling references
- ✅ **Parallel Execution**: Sui's transaction parallelization for high throughput
- ✅ **Event Emissions**: Comprehensive event system for off-chain indexing
- ✅ **Balance<SUI> Integration**: Native SUI token handling
- ✅ **Keccak256 Hashing**: HTLC support with hash verification

## Project Statistics

```
Total Modules:           10
Total Lines of Code:     1,917 lines (Move)
Language:               Move (Sui Framework)
Blockchain:             Sui

Module Breakdown:
  SignerList:           244 lines (Weighted multisig with proposals)
  AccountDelete:        226 lines (Lifecycle management)
  TrustLines:           222 lines (Bilateral credit networks)
  DepositAuth:          211 lines (Multi-tier KYC/AML)
  Escrow:               187 lines (HTLC with Keccak256)
  Checks:               171 lines (Deferred payments)
  DEXOrders:            170 lines (On-chain orderbook)
  DepositPreauth:       165 lines (One-time tokens)
  PaymentChannels:      163 lines (Streaming micropayments)
  DIDManager:           158 lines (W3C DID standard)
```

## Architecture Patterns

### Sui-Specific Optimizations

1. **Shared Objects**: Registries use `transfer::share_object()` for multi-user access
2. **Owned Objects**: Individual items like Channels, Escrow, Checks are shared objects
3. **Balance<SUI>**: Native Sui balance type for gas-efficient fund management
4. **Table Storage**: Dynamic key-value storage for scalable collections
5. **VecMap**: Efficient small map implementation for signer lists

### Move Language Features

- **Resource Safety**: Objects cannot be copied or dropped accidentally
- **Generics**: Type-safe generic programming (preparing for multi-asset support)
- **Abilities**: Explicit control over copy, drop, store, and key capabilities
- **Entry Functions**: Direct transaction invocation from clients

## The 10 Financial Primitives

### 1. TrustLines (`trust_lines.move`)

Bilateral credit lines with payment rippling for multi-hop credit networks.

**Key Features**:
- Deterministic key generation from ordered addresses
- Bidirectional balance tracking with `is_negative` flag
- Credit limit enforcement for both directions
- Payment rippling support

**Usage**:
```move
// Create trust line
sui client call --function create_trust_line \
  --module trust_lines \
  --args $REGISTRY_ID $COUNTERPARTY 1000000000 \
  --gas-budget 10000000

// Ripple payment
sui client call --function ripple_payment \
  --module trust_lines \
  --args $REGISTRY_ID $RECEIVER 500000000 \
  --gas-budget 10000000
```

### 2. PaymentChannels (`payment_channels.move`)

Streaming micropayments with off-chain efficiency and on-chain settlement.

**Key Features**:
- Channel object with `Balance<SUI>` for deposits
- Incremental claims by receiver
- Expiration-based security
- Sender can close and reclaim remaining funds

**Usage**:
```move
// Create channel (attach SUI coin)
sui client call --function create_channel \
  --module payment_channels \
  --args $COIN $RECEIVER $EXPIRATION \
  --gas-budget 10000000

// Claim funds
sui client call --function claim_funds \
  --module payment_channels \
  --args $CHANNEL_ID 100000000 \
  --gas-budget 10000000
```

### 3. Escrow (`escrow.move`)

Time-locked and hash-locked conditional payments (HTLC).

**Key Features**:
- Dual time windows: `release_time` and `cancel_time`
- Optional `condition_hash` for HTLC
- Keccak256 preimage verification
- Separate functions for time-locked vs hash-locked creation

**Usage**:
```move
// Create hash-locked escrow (HTLC)
sui client call --function create_hash_locked \
  --module escrow \
  --args $COIN $RECEIVER $RELEASE_TIME $CANCEL_TIME $HASH \
  --gas-budget 10000000

// Execute with preimage
sui client call --function execute_escrow \
  --module escrow \
  --args $ESCROW_ID $PREIMAGE \
  --gas-budget 10000000
```

### 4. Checks (`checks.move`)

Deferred payments that recipients can cash later, with partial cashing support.

**Key Features**:
- Partial cashing with `cashed_amount` tracking
- Automatic status change to `CASHED` when fully claimed
- Sender can cancel before expiration
- Anyone can trigger expiration after timeout

**Usage**:
```move
// Create check
sui client call --function create_check \
  --module checks \
  --args $COIN $RECEIVER $EXPIRATION \
  --gas-budget 10000000

// Cash check (partial amount)
sui client call --function cash_check \
  --module checks \
  --args $CHECK_ID 250000000 \
  --gas-budget 10000000
```

### 5. DEXOrders (`dex_orders.move`)

On-chain orderbook with limit orders and partial fills.

**Key Features**:
- Proportional payment calculation for partial fills
- Status tracking: Open → PartiallyFilled → Filled
- Automatic fund transfers to both parties
- Cancel with remaining fund refund

**Usage**:
```move
// Place order
sui client call --function place_order \
  --module dex_orders \
  --args $COIN "SUI" "USDC" 1000000000 \
  --gas-budget 10000000

// Fill order
sui client call --function fill_order \
  --module dex_orders \
  --args $ORDER_ID 500000000 $PAYMENT_COIN \
  --gas-budget 10000000
```

### 6. DIDManager (`did_manager.move`)

Decentralized identifier management following W3C DID standards.

**Key Features**:
- Bidirectional mapping: DID ↔ Account
- One DID per account enforcement
- Active/revoked status tracking
- Document URI updates

**Usage**:
```move
// Register DID
sui client call --function register_did \
  --module did_manager \
  --args $REGISTRY_ID "did:sui:12345" "https://example.com/did.json" \
  --gas-budget 10000000

// Update DID document
sui client call --function update_did \
  --module did_manager \
  --args $REGISTRY_ID "https://example.com/did-v2.json" \
  --gas-budget 10000000
```

### 7. DepositAuthorization (`deposit_authorization.move`)

Multi-tier KYC/AML compliance framework for authorized deposits.

**Key Features**:
- 4 KYC tiers: Basic, Standard, Premium, Institutional
- Tier-based amount limits (1, 10, 100, 1000 SUI)
- Usage tracking with `used_amount`
- Expiration and revocation support

**Usage**:
```move
// Create authorization (Premium tier)
sui client call --function create_authorization \
  --module deposit_authorization \
  --args $REGISTRY $AUTHORIZED "SUI" 100000000000 $EXPIRATION 2 \
  --gas-budget 10000000

// Use authorization
sui client call --function use_authorization \
  --module deposit_authorization \
  --args $REGISTRY $AUTHORIZER "SUI" 50000000000 \
  --gas-budget 10000000
```

### 8. DepositPreauth (`deposit_preauth.move`)

One-time pre-authorization tokens for specific deposits.

**Key Features**:
- Single-use tokens with unique IDs
- Status: Active → Used/Revoked
- Expiration enforcement
- Authorizer can revoke before use

**Usage**:
```move
// Create preauth
sui client call --function create_preauth \
  --module deposit_preauth \
  --args $REGISTRY $AUTHORIZED "SUI" 1000000000 $EXPIRATION \
  --gas-budget 10000000

// Use preauth
sui client call --function use_preauth \
  --module deposit_preauth \
  --args $REGISTRY $PREAUTH_ID 1000000000 \
  --gas-budget 10000000
```

### 9. SignerList (`signer_list.move`)

Weighted multisig with proposal-based governance.

**Key Features**:
- VecMap for efficient signer → weight mapping
- Quorum as basis points (0-10000 = 0%-100%)
- Proposal system with approval tracking
- Weighted voting with automatic quorum checking

**Usage**:
```move
// Create signer list (60% quorum)
sui client call --function create_signer_list \
  --module signer_list \
  --args 6000 \
  --gas-budget 10000000

// Add signer with 30% weight
sui client call --function add_signer \
  --module signer_list \
  --args $LIST_ID $SIGNER 3000 \
  --gas-budget 10000000

// Create and approve proposal
sui client call --function create_proposal \
  --module signer_list \
  --args $REGISTRY $LIST_ID "Upgrade contract" \
  --gas-budget 10000000

sui client call --function approve_proposal \
  --module signer_list \
  --args $REGISTRY $LIST_ID $PROPOSAL_ID \
  --gas-budget 10000000
```

### 10. AccountDelete (`account_delete.move`)

Account lifecycle management with grace period and fund recovery.

**Key Features**:
- 24-hour grace period (3600 epochs)
- Status: Active → PendingDeletion → Deleted
- Beneficiary designation for fund transfer
- Cancellation support during grace period

**Usage**:
```move
// Create account
sui client call --function create_account \
  --module account_delete \
  --args $REGISTRY \
  --gas-budget 10000000

// Request deletion
sui client call --function request_deletion \
  --module account_delete \
  --args $REGISTRY $BENEFICIARY \
  --gas-budget 10000000

// Execute deletion (after grace period)
sui client call --function execute_deletion \
  --module account_delete \
  --args $REGISTRY $ACCOUNT_ID \
  --gas-budget 10000000
```

## Getting Started

### Prerequisites

```bash
# Install Sui CLI
cargo install --locked --git https://github.com/MystenLabs/sui.git --branch mainnet sui

# Verify installation
sui --version
```

### Building

```bash
cd sui/
sui move build
```

### Testing

```bash
sui move test
```

### Deployment

```bash
# Publish to devnet
sui client publish --gas-budget 100000000

# Publish to testnet
sui client publish --gas-budget 100000000 --network testnet

# Publish to mainnet
sui client publish --gas-budget 100000000 --network mainnet
```

## Technical Deep Dive

### Object Model

Sui's object-centric model enables unique patterns:

- **Shared Objects**: Registries accessible by multiple users
- **Transfer Semantics**: Objects can be transferred, shared, or frozen
- **Parallel Execution**: Non-conflicting transactions execute in parallel

### Storage Patterns

1. **Table**: Dynamic key-value storage (`trust_lines`, `did_manager`)
2. **VecMap**: Small map optimization (`signer_list`)
3. **Balance<T>**: Type-safe token balances (`payment_channels`, `escrow`)

### Event System

All modules emit events for off-chain indexing:

```move
public struct TrustLineCreated has copy, drop {
    account1: address,
    account2: address,
    limit: u64,
}

event::emit(TrustLineCreated { account1, account2, limit });
```

## Security Considerations

### Authorization

- All mutable operations verify `tx_context::sender(ctx)`
- Status checks prevent operations on inactive/deleted objects
- Expiration enforcement for time-sensitive primitives

### Resource Safety

- Move's type system prevents:
  - Double-spending
  - Unauthorized copying
  - Resource leaks
  - Dangling references

### Reentrancy Protection

- Move's single-writer semantics prevent reentrancy
- No callbacks or cross-contract calls during execution

## Comparison to Other Chains

| Feature | Sui | Aptos | Ethereum |
|---------|-----|-------|----------|
| Object Model | Owned/Shared Objects | Global Storage | Account Storage |
| Parallelization | Native | AptosBFT | Sequential |
| Move Language | Sui Move | Aptos Move | Solidity |
| Gas Model | Object-based | Resource-based | Opcode-based |
| Consensus | Narwhal + Bullshark | AptosBFT | PoS |

## Advanced Usage

### Multi-Asset Support

Modules are designed to support generic assets (future):

```move
// Future enhancement
public struct Channel<phantom T> has key {
    id: UID,
    balance: Balance<T>,  // Generic asset type
    ...
}
```

### Composability

Primitives can be composed:

```move
// Example: TrustLine-backed payment channel
// 1. Create trust line for credit
// 2. Create payment channel for streaming
// 3. Settle via trust line rippling
```

## Performance Characteristics

- **TrustLines**: O(1) lookups, O(1) ripple payments
- **PaymentChannels**: O(1) claims, minimal storage
- **Escrow**: O(1) execution with optional hash verification
- **SignerList**: O(n) where n = number of signers (small)

## Roadmap

- [x] All 10 primitives implemented
- [ ] Unit tests with Move testing framework
- [ ] Integration tests
- [ ] Mainnet deployment
- [ ] SDK for TypeScript/Rust
- [ ] Multi-asset support (Coin<T> generics)

## Resources

- **Sui Documentation**: https://docs.sui.io
- **Move Language**: https://move-language.github.io
- **Sui SDK**: https://github.com/MystenLabs/sui/tree/main/sdk

## License

MIT License - See [LICENSE](../LICENSE) for details.

## Acknowledgments

- **Sui Foundation** - For the innovative object-centric blockchain
- **Move Language Team** - For resource-oriented programming
- **XRPL Community** - For pioneering these financial primitives

---

**Built with ❤️ for the Sui ecosystem**
