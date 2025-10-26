#!/bin/bash

# Optimize all CosmWasm contracts for deployment
# Usage: ./scripts/optimize.sh

set -e

echo "Optimizing all XRPL Financial Primitive contracts..."
echo ""

# Check if docker is installed
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is required for optimization"
    echo "Please install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

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

# Create artifacts directory
mkdir -p artifacts

for contract in "${CONTRACTS[@]}"; do
  echo "Optimizing $contract..."

  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/rust-optimizer:0.15.0 \
    ./contracts/$contract

  echo "✅ $contract optimized"
  echo ""
done

echo "🎉 All contracts optimized!"
echo ""
echo "Optimized .wasm files are in the artifacts/ directory"
echo "Ready for deployment to testnet/mainnet"
