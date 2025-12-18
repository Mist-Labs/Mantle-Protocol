#!/bin/bash
set -euo pipefail

# Load env vars safely if .env exists
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

: "${WEBHOOK_URL:?WEBHOOK_URL is not set}"
: "${GOLDSKY_WEBHOOK_SECRET:?GOLDSKY_WEBHOOK_SECRET is not set}"

echo "Webhook configured"
echo "🚀 Creating Goldsky Webhooks..."
echo "Webhook URL: $WEBHOOK_URL"
echo ""

################################
# Mantle Sepolia Webhooks
################################
echo "📡 Creating Mantle webhooks..."

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v2 \
  --name mantle-intent-created \
  --entity intent_created \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v2 \
  --name mantle-intent-filled \
  --entity intent_filled \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v2 \
  --name mantle-intent-refunded \
  --entity intent_refunded \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v2 \
  --name mantle-withdrawal-claimed \
  --entity withdrawal_claimed \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-mantle-mantle-sepolia/v2 \
  --name mantle-root-synced \
  --entity root_synced \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

################################
# Ethereum Sepolia Webhooks
################################
echo ""
echo "📡 Creating Ethereum webhooks..."

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v2 \
  --name ethereum-intent-created \
  --entity intent_created \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v2 \
  --name ethereum-intent-filled \
  --entity intent_filled \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v2 \
  --name ethereum-intent-refunded \
  --entity intent_refunded \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v2 \
  --name ethereum-withdrawal-claimed \
  --entity withdrawal_claimed \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

goldsky subgraph webhook create shadowswap-ethereum-sepolia/v2 \
  --name ethereum-root-synced \
  --entity root_synced \
  --url "$WEBHOOK_URL" \
  --secret "$GOLDSKY_WEBHOOK_SECRET"

################################
# Done
################################
echo ""
echo "✅ All webhooks created successfully!"
echo ""
echo "📝 List webhooks:"
echo "  goldsky subgraph webhook list shadowswap-mantle-mantle-sepolia/v2"
echo "  goldsky subgraph webhook list shadowswap-ethereum-sepolia/v2"
echo ""
echo "🧪 Test webhook:"
echo "  curl -X POST $WEBHOOK_URL -H 'Content-Type: application/json' -d '{\"test\":true}'"
