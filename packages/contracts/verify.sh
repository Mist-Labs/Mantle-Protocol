#!/bin/bash
set -e

source .env

echo "🔍 Verifying PoseidonHasher on Mantle..."
forge verify-contract 0x8EA86eD4317AF92f73E5700eB9b93A72dE62f3B1 \
  src/poseidonHasher.sol:PoseidonHasher \
  --rpc-url $MANTLE_RPC_URL \
  --etherscan-api-key $MANTLESCAN_API_KEY \
  --watch
sleep 30

echo "🔍 Verifying Mantle PrivateIntentPool..."
forge verify-contract 0x83B1F9aA4B572edE7db24bE5D770272B1d375e07 \
  src/privateIntentPool.sol:PrivateIntentPool \
  --rpc-url $MANTLE_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address)" \
    $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $MANTLESCAN_API_KEY \
  --watch
sleep 30

echo "🔍 Verifying Mantle PrivateSettlement..."
forge verify-contract 0x0Be1C31a27F6477dd5DeB4eC4302B4cF199362CF \
  src/privateSettlement.sol:PrivateSettlement \
  --rpc-url $MANTLE_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address)" \
    $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $MANTLESCAN_API_KEY \
  --watch
sleep 30

echo "🔍 Verifying Ethereum PrivateIntentPool..."
forge verify-contract 0x75e3a5461eAa204a1fce8b54De3cf572aEEA9504 \
  src/privateIntentPool.sol:PrivateIntentPool \
  --rpc-url $ETHEREUM_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address)" \
    $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $ETHERSCAN_API_KEY \
  --watch
sleep 30

echo "🔍 Verifying Ethereum PrivateSettlement..."
forge verify-contract 0x1DC568D1B13C513D220212DdaA6897aAD06C05F0 \
  src/privateSettlement.sol:PrivateSettlement \
  --rpc-url $ETHEREUM_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address)" \
    $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $ETHERSCAN_API_KEY \
  --watch

echo "✅ All contracts verified!"