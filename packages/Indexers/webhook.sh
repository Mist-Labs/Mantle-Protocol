#!/bin/bash

set -e

# Load environment variables
source .env

WEBHOOK_URL="${WEBHOOK_URL:-https://028f7e776cdd.ngrok-free.app/webhook}"
WEBHOOK_SECRET="${GOLDSKY_WEBHOOK_SECRET:-d27a15ad2c2ebb7779421c1da4a29d09a424df5090bf886cf1dd3b2def0cf843}"

echo "🚀 Creating Goldsky Webhooks..."
echo "Webhook URL: $WEBHOOK_URL"
echo ""

# Mantle Sepolia Webhooks (using lowercase snake_case entity names)
echo "📡 Creating Mantle webhooks..."

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v1 \
  --name mantle-intent-created \
  --entity intent_created \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v1 \
  --name mantle-intent-filled \
  --entity intent_filled \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v1 \
  --name mantle-intent-refunded \
  --entity intent_refunded \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v1 \
  --name mantle-withdrawal-claimed \
  --entity withdrawal_claimed \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v1 \
  --name mantle-root-synced \
  --entity root_synced \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

echo ""
echo "📡 Creating Ethereum webhooks..."

# Ethereum Sepolia Webhooks (using lowercase snake_case entity names)
goldsky subgraph webhook create shadowswap-ethereum-sepolia/v1 \
  --name ethereum-intent-created \
  --entity intent_created \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v1 \
  --name ethereum-intent-filled \
  --entity intent_filled \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v1 \
  --name ethereum-intent-refunded \
  --entity intent_refunded \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v1 \
  --name ethereum-withdrawal-claimed \
  --entity withdrawal_claimed \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v1 \
  --name ethereum-root-synced \
  --entity root_synced \
  --url "$WEBHOOK_URL" \
  --secret "$WEBHOOK_SECRET"

echo ""
echo "✅ All webhooks created successfully!"
echo ""
echo "📝 List webhooks:"
echo "  goldsky subgraph webhook list shadowswap-mantle-mantle-sepolia/v1"
echo "  goldsky subgraph webhook list shadowswap-ethereum-sepolia/v1"
echo ""
echo "📝 Test webhook:"
echo "  curl -X POST $WEBHOOK_URL -H 'Content-Type: application/json' -d '{\"test\":true}'"