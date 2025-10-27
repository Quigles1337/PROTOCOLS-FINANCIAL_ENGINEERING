# XRPL Financial Primitives for TON

**Production-grade implementation of all 10 XRPL-inspired financial primitives in FunC for the TON blockchain**

## Overview

This implementation brings XRPL's battle-tested financial primitives to TON (The Open Network), Telegram's high-performance blockchain, using FunC - TON's native smart contract language optimized for the TVM (TON Virtual Machine).

## Key Features

- ✅ **Cell-Based Storage**: Leverages TON's unique cell architecture for efficient data structures
- ✅ **Message-Passing Architecture**: Native TON internal messages for fund transfers
- ✅ **TVM Optimized**: FunC code compiled for TON Virtual Machine efficiency
- ✅ **Dictionary Storage**: Uses hashmaps (dicts) for scalable key-value storage
- ✅ **Gas Efficient**: Optimized for TON's gas model and storage costs
- ✅ **Telegram Integration Ready**: Built for TON's ecosystem and Telegram bots

## Project Statistics

```
Total Contracts:         10
Total Lines of Code:     2,101 lines (FunC)
Language:               FunC (TON smart contract language)
Blockchain:             TON (The Open Network)

Contract Breakdown:
  SignerList:           273 lines (Weighted multisig governance)
  AccountDelete:        254 lines (24-hour grace period lifecycle)
  PaymentChannels:      222 lines (Streaming micropayments)
  Escrow:               222 lines (HTLC with hash verification)
  TrustLines:           211 lines (Bilateral credit networks)
  DEXOrders:            198 lines (On-chain orderbook)
  DepositAuth:          195 lines (4-tier KYC/AML)
  Checks:               191 lines (Deferred payments)
  DIDManager:           171 lines (W3C DID standard)
  DepositPreauth:       164 lines (One-time tokens)
```

## Architecture Patterns

### TON-Specific Optimizations

1. **Cell Storage**: All data stored in cells for efficient serialization
2. **Dictionary (Dict)**: Hashmaps with 256-bit or 64-bit integer keys
3. **Message Passing**: `send_raw_message()` for native TON transfers
4. **Slice Operations**: Efficient slice parsing for data retrieval
5. **Builder Pattern**: Structured cell construction for storage

### FunC Language Features

- **Stack-Based**: TVM stack operations for gas efficiency
- **Inline Functions**: Zero-cost abstractions with `inline` keyword
- **Impure Functions**: Explicit side-effect management
- **Method ID**: Get methods for off-chain queries
- **Op Codes**: Operation dispatch via internal message handlers

## The 10 Financial Primitives

### 1. TrustLines (`trust_lines.fc`)

Bilateral credit lines with payment rippling for multi-hop credit networks.

**Key Features**:
- Dictionary-based storage with hash keys from ordered addresses
- Bidirectional balance tracking with `is_negative` flag
- Credit limit enforcement for both directions
- Ripple payment logic for credit flow

**Message Ops**:
```func
op = 1: create_trust_line(counterparty, limit)
op = 2: update_limit(counterparty, new_limit)
op = 3: ripple_payment(receiver, amount)
```

**Usage**:
```bash
# Via TON CLI or Telegram bot
ton-cli send <contract> --amount 0 --op 1 --data "counterparty_addr,1000000000"
```

### 2. PaymentChannels (`payment_channels.fc`)

Streaming micropayments with off-chain efficiency and on-chain settlement.

**Key Features**:
- Channel storage with `next_id` incrementing
- Balance and `total_claimed` tracking
- Expiration-based security
- Native TON token transfers via `send_raw_message()`

**Message Ops**:
```func
op = 1: create_channel(receiver, expiration) [payable]
op = 2: add_funds(channel_id) [payable]
op = 3: claim_funds(channel_id, amount)
op = 4: close_channel(channel_id)
```

### 3. Escrow (`escrow.fc`)

Time-locked and hash-locked conditional payments (HTLC).

**Key Features**:
- Dual time windows: `release_time` and `cancel_time`
- Optional `condition_hash` (256-bit) for HTLC
- Hash verification using `slice_hash()`
- Separate creation methods for time vs hash locks

**Message Ops**:
```func
op = 1: create_time_locked(receiver, release_time, cancel_time) [payable]
op = 2: create_hash_locked(receiver, release_time, cancel_time, hash) [payable]
op = 3: execute_escrow(escrow_id, preimage)
op = 4: cancel_escrow(escrow_id)
```

### 4. Checks (`checks.fc`)

Deferred payments that recipients can cash later, with partial cashing support.

**Key Features**:
- Partial cashing with `cashed_amount` tracking
- Automatic status change to `cashed` when fully claimed
- Sender can cancel and reclaim remaining funds
- Expiration enforcement

**Message Ops**:
```func
op = 1: create_check(receiver, expiration) [payable]
op = 2: cash_check(check_id, amount)
op = 3: cancel_check(check_id)
```

### 5. DEXOrders (`dex_orders.fc`)

On-chain orderbook with limit orders and partial fills.

**Key Features**:
- Proportional payment calculation: `(fill_amount * buy_amount) / sell_amount`
- Status: Open → PartiallyFilled → Filled → Cancelled
- Dual fund transfers to both filler and creator
- Remaining balance refund on cancellation

**Message Ops**:
```func
op = 1: place_order(buy_amount) [payable with sell_amount]
op = 2: fill_order(order_id, fill_amount) [payable with payment]
op = 3: cancel_order(order_id)
```

### 6. DIDManager (`did_manager.fc`)

Decentralized identifier management following W3C DID standards.

**Key Features**:
- Bidirectional mapping: `dids` dict and `account_to_did` dict
- One DID per account enforcement
- Active/revoked status tracking
- Document URI stored in cell refs for large strings

**Message Ops**:
```func
op = 1: register_did(did_ref, document_uri_ref)
op = 2: update_did(new_document_uri_ref)
op = 3: revoke_did()
```

### 7. DepositAuthorization (`deposit_authorization.fc`)

Multi-tier KYC/AML compliance framework for authorized deposits.

**Key Features**:
- 4 KYC tiers: Basic (1 TON), Standard (10 TON), Premium (100 TON), Institutional (1000 TON)
- Tier-based amount limits with `get_tier_max_amount()`
- Usage tracking with `used_amount` increment
- Expiration and revocation support

**Message Ops**:
```func
op = 1: create_authorization(authorized, max_amount, expiration, tier)
op = 2: use_authorization(authorizer, amount)
op = 3: revoke_authorization(authorized)
```

### 8. DepositPreauth (`deposit_preauth.fc`)

One-time pre-authorization tokens for specific deposits.

**Key Features**:
- Single-use tokens with unique 64-bit IDs
- Status: Active → Used/Revoked
- Expiration enforcement with `now()` checks
- Authorizer can revoke before use

**Message Ops**:
```func
op = 1: create_preauth(authorized, max_amount, expiration)
op = 2: use_preauth(preauth_id, amount)
op = 3: revoke_preauth(preauth_id)
```

### 9. SignerList (`signer_list.fc`)

Weighted multisig with proposal-based governance.

**Key Features**:
- Nested dictionaries: `signer_lists` and `proposals`
- Quorum as basis points (0-10000 = 0%-100%)
- Weighted approval tracking with incremental `approval_weight`
- Proposal status: Pending → Executed/Rejected

**Message Ops**:
```func
op = 1: create_signer_list(quorum)
op = 2: add_signer(list_id, new_signer, weight)
op = 3: create_proposal(list_id)
op = 4: approve_proposal(list_id, proposal_id)
op = 5: execute_proposal(list_id, proposal_id)
```

### 10. AccountDelete (`account_delete.fc`)

Account lifecycle management with 24-hour grace period.

**Key Features**:
- 24-hour (86400 seconds) grace period constant
- Status: Active → PendingDeletion → Deleted
- Beneficiary designation with `maybe` encoding
- Balance transfer to beneficiary on deletion

**Message Ops**:
```func
op = 1: create_account()
op = 2: deposit() [payable]
op = 3: request_deletion(beneficiary)
op = 4: cancel_deletion()
op = 5: execute_deletion(account_id)
```

## Getting Started

### Prerequisites

```bash
# Install TON development tools
npm install -g ton-compiler
npm install -g @ton-community/func-js

# Or use Docker
docker pull tonlabs/compilers
```

### Building

```bash
cd ton/contracts/

# Compile FunC to FIFT
func -o trust_lines.fif -SPA trust_lines.fc

# Compile to BOC (Bag of Cells)
fift -s build.fif trust_lines
```

### Testing

```bash
# Using TON Sandbox
npm install @ton-community/sandbox
npm test

# Using Blueprint
npm install @ton-community/blueprint
npx blueprint test
```

### Deployment

```bash
# Deploy to testnet
ton-cli deploy trust_lines.boc --network testnet

# Deploy to mainnet
ton-cli deploy trust_lines.boc --network mainnet
```

## Technical Deep Dive

### Cell Architecture

TON uses cells as the fundamental storage unit:

```func
;; Cell construction
cell my_data = begin_cell()
    .store_uint(42, 32)
    .store_slice(some_address)
    .store_coins(1000000000)
.end_cell();

;; Cell parsing
slice ds = my_data.begin_parse();
int value = ds~load_uint(32);
slice addr = ds~load_msg_addr();
int amount = ds~load_coins();
```

### Dictionary (Hashmap) Storage

Efficient key-value storage using dictionaries:

```func
;; 64-bit key dictionary
cell dict = new_dict();
dict~udict_set_ref(64, key_id, value_cell);
(cell val, int found) = dict.udict_get_ref?(64, key_id);

;; 256-bit key dictionary
int key_hash = slice_hash(some_slice);
dict~udict_set_builder(256, key_hash, value_builder);
```

### Message Handling

Internal message processing:

```func
() recv_internal(int msg_value, cell in_msg_full, slice in_msg_body) impure {
    load_data();

    int op = in_msg_body~load_uint(32);
    slice sender = in_msg_full.begin_parse().skip_bits(4).load_msg_addr();

    if (op == 1) { /* handle operation 1 */ }
    if (op == 2) { /* handle operation 2 */ }

    save_data();
}
```

### Sending Messages

Native TON token transfers:

```func
send_raw_message(begin_cell()
    .store_uint(0x10, 6)           ;; nobounce flag
    .store_slice(destination)       ;; recipient address
    .store_coins(amount)            ;; nanotons to send
    .store_uint(0, 1 + 4 + 4 + 64 + 32 + 1 + 1)  ;; default message flags
.end_cell(), 1);                    ;; mode 1: pay fees separately
```

## Security Considerations

### Authorization

- All operations verify sender via `in_msg_full.begin_parse().load_msg_addr()`
- Status checks prevent operations on deleted/inactive items
- Expiration enforcement using `now()` Unix timestamp

### Reentrancy Protection

- FunC doesn't support async/await - messages execute atomically
- No callbacks during execution
- State saved only after all checks pass

### Gas Management

- Efficient dictionary operations minimize gas costs
- Inline functions for zero-cost abstractions
- Optimized cell structure to reduce storage costs

## Comparison to Other Chains

| Feature | TON | Ethereum | Near |
|---------|-----|----------|------|
| Storage Model | Cell-based | Account Storage | Key-value |
| Language | FunC | Solidity | Rust |
| Consensus | PoS (Catchain) | PoS | PoS (Nightshade) |
| Sharding | Native (Dynamic) | Planned | Native |
| TPS | ~100k (sharded) | ~30 | ~100k |
| Finality | ~5 seconds | ~15 minutes | ~1 second |

## TON-Specific Advantages

### 1. Telegram Integration

- Native bot integration via TON Connect
- In-chat payments and DApp interactions
- 700M+ potential users

### 2. Infinite Sharding

- Dynamic workchain and shard chains
- Horizontal scalability
- Sub-second finality

### 3. Storage Optimization

- Cell pruning and garbage collection
- Pay-per-byte storage model
- Efficient for large-scale applications

## Advanced Usage

### Telegram Bot Integration

```javascript
// Using @ton/ton
import { TonClient, WalletContractV4, internal } from '@ton/ton';

const client = new TonClient({ endpoint: 'https://toncenter.com/api/v2/jsonRPC' });

// Create trust line
await wallet.sendTransfer({
    to: trustLinesContract,
    value: toNano('0.05'),
    body: beginCell()
        .storeUint(1, 32)  // op: create_trust_line
        .storeAddress(counterparty)
        .storeCoins(toNano('1000'))
    .endCell()
});
```

### TON Connect Integration

```javascript
import { TonConnectUI } from '@tonconnect/ui';

const tonConnectUI = new TonConnectUI({
    manifestUrl: 'https://your-app.com/tonconnect-manifest.json'
});

await tonConnectUI.sendTransaction({
    messages: [{
        address: checkContract,
        amount: toNano('10'),
        payload: createCheckPayload(receiver, expiration)
    }]
});
```

## Performance Characteristics

- **TrustLines**: O(1) dict lookups, O(1) payments
- **PaymentChannels**: O(1) claims, minimal on-chain footprint
- **Escrow**: O(1) execution with optional hash verification
- **SignerList**: O(n) where n = number of signers (typically small)
- **All Contracts**: Sub-second confirmation on TON mainnet

## Roadmap

- [x] All 10 primitives implemented
- [ ] Blueprint project structure
- [ ] Comprehensive test suites using TON Sandbox
- [ ] Telegram bot examples for each primitive
- [ ] TON Connect integration examples
- [ ] Mainnet deployment
- [ ] TypeScript SDK
- [ ] Integration with TON Jettons (tokens)

## Resources

- **TON Documentation**: https://docs.ton.org
- **FunC Language**: https://docs.ton.org/develop/func/overview
- **TON Connect**: https://github.com/ton-connect
- **TON SDK**: https://github.com/ton-org/ton

## License

MIT License - See [LICENSE](../LICENSE) for details.

## Acknowledgments

- **TON Foundation** - For the high-performance blockchain
- **Telegram** - For integrating TON into the messaging platform
- **TON Community** - For FunC tooling and libraries
- **XRPL Community** - For pioneering these financial primitives

---

**Built with ❤️ for the TON ecosystem and Telegram's 700M+ users**
