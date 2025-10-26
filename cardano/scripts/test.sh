#!/bin/bash
# Run all Aiken tests

set -e

echo "Running tests for all XRPL Financial Primitives..."
echo ""

# Run tests
aiken check

echo ""
echo "✅ All tests passed!"
echo ""
echo "For coverage: aiken check --trace"
