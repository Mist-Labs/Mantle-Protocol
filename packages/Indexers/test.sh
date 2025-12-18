#!/bin/bash
set -euo pipefail

: "${WEBHOOK_URL:?WEBHOOK_URL is not set}"
: "${GOLDSKY_WEBHOOK_SECRET:?GOLDSKY_WEBHOOK_SECRET is not set}"

echo "Webhook configured"
echo "🧪 Testing webhook with Goldsky secret..."
echo "URL: $WEBHOOK_URL"
echo ""

# Test payload
PAYLOAD='{
  "event_type":"intent_created",
  "intent_id":"0x1234",
  "chain":"ethereum",
  "transaction_hash":"0xabcd",
  "block_number":"12345",
  "timestamp":"1234567890"
}'

curl -X POST "$WEBHOOK_URL" \
  -H "Content-Type: application/json" \
  -H "goldsky-webhook-secret: $GOLDSKY_WEBHOOK_SECRET" \
  -d "$PAYLOAD" \
  -v

echo ""
echo "✅ Test complete!"
