#!/bin/bash

set -e

source .env

echo "🚀 Deploying Goldsky Subgraphs..."

# Deploy Ethereum Sepolia
echo ""
echo "📡 Deploying Ethereum Sepolia subgraph..."
goldsky subgraph deploy shadowswap-ethereum/v3 \
  --from-abi ./goldsky-config-ethereum.json

# Deploy Mantle Sepolia
echo ""
echo "📡 Deploying Mantle Sepolia subgraph..."
goldsky subgraph deploy shadowswap-mantle/v3 \
  --from-abi ./goldsky-config-mantle.json

echo ""
echo "✅ All subgraphs deployed successfully!"
echo ""
echo "Check status:"
echo "  goldsky subgraph status shadowswap-ethereum/"
echo "  goldsky subgraph status shadowswap-mantle/"