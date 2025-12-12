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
forge verify-contract 0xa35Ef7650e3E192f63182F1dDe8f162Ba847CB12 \
  src/privateIntentPool.sol:PrivateIntentPool \
  --rpc-url $MANTLE_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address,address)" \
    $OWNER_ADDRESS $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $MANTLESCAN_API_KEY \
  --watch
sleep 30

echo "🔍 Verifying Mantle PrivateSettlement..."
forge verify-contract 0x28650373758d75a8fF0B22587F111e47BAC34e21 \
  src/privateSettlement.sol:PrivateSettlement \
  --rpc-url $MANTLE_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address,address)" \
     $OWNER_ADDRESS $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $MANTLESCAN_API_KEY \
  --watch
sleep 30

echo "🔍 Verifying Ethereum PrivateIntentPool..."
forge verify-contract 0x2D7102132042f60390AE76a24bF4Bd4358184dA3 \
  src/privateIntentPool.sol:PrivateIntentPool \
  --rpc-url $ETHEREUM_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address,address)" \
     $OWNER_ADDRESS $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $ETHERSCAN_API_KEY \
  --watch
sleep 30

echo "🔍 Verifying Ethereum PrivateSettlement..."
forge verify-contract 0x77cd62B23ADe926355C6BaA35832C498Dc8c2E6F \
  src/privateSettlement.sol:PrivateSettlement \
  --rpc-url $ETHEREUM_RPC_URL \
  --constructor-args $(cast abi-encode "constructor(address,address,address,address)" \
     $OWNER_ADDRESS $RELAYER_ADDRESS $FEE_COLLECTOR_ADDRESS $POSEIDON_HASHER_ADDRESS) \
  --etherscan-api-key $ETHERSCAN_API_KEY \
  --watch

echo "✅ All contracts verified!"