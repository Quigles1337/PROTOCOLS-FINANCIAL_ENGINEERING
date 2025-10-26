#!/bin/bash
# Build all Aiken validators

set -e

echo "Building all XRPL Financial Primitive validators for Cardano..."
echo ""

# Build all contracts
aiken build

echo ""
echo "✅ All validators compiled successfully!"
echo ""
echo "Validators are in plutus.json"
echo "To run tests: ./scripts/test.sh"
