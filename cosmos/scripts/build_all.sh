#!/bin/bash

# Build all CosmWasm contracts
# Usage: ./scripts/build_all.sh

set -e

echo "Building all XRPL Financial Primitive contracts for CosmWasm..."
echo ""

CONTRACTS=(
  "trust-lines"
  "payment-channels"
  "escrow"
  "checks"
  "dex-orders"
  "did-manager"
  "deposit-authorization"
  "deposit-preauth"
  "signer-list"
  "account-delete"
)

for contract in "${CONTRACTS[@]}"; do
  echo "Building $contract..."
  cd contracts/$contract
  cargo wasm
  cd ../..
  echo "✅ $contract built successfully"
  echo ""
done

echo "🎉 All contracts built successfully!"
echo ""
echo "To optimize for deployment, run: ./scripts/optimize.sh"
