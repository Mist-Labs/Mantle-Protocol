#!/bin/bash

set -e

echo "🚀 Deploying Goldsky Subgraphs..."

# Deploy Ethereum Sepolia
echo ""
echo "📡 Deploying Ethereum Sepolia subgraph..."
goldsky subgraph deploy shadowswap-ethereum/v1 \
  --from-abi ./goldsky-config-ethereum.json

# Deploy Mantle Sepolia
echo ""
echo "📡 Deploying Mantle Sepolia subgraph..."
goldsky subgraph deploy shadowswap-mantle/v1 \
  --from-abi ./goldsky-config-mantle.json

echo ""
echo "✅ All subgraphs deployed successfully!"
echo ""
echo "Check status:"
echo "  goldsky subgraph status shadowswap-ethereum/v1"
echo "  goldsky subgraph status shadowswap-mantle/v1"