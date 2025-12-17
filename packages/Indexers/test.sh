#!/bin/bash

WEBHOOK_URL="${WEBHOOK_URL:-https://028f7e776cdd.ngrok-free.app/webhook}"
WEBHOOK_SECRET="${GOLDSKY_WEBHOOK_SECRET:-d27a15ad2c2ebb7779421c1da4a29d09a424df5090bf886cf1dd3b2def0cf843}"

# Test payload
PAYLOAD='{"event_type":"intent_created","intent_id":"0x1234","chain":"ethereum","transaction_hash":"0xabcd","block_number":"12345","timestamp":"1234567890"}'

echo "🧪 Testing webhook with Goldsky secret..."
echo "URL: $WEBHOOK_URL"
echo "Secret: $WEBHOOK_SECRET"
echo ""

curl -X POST "$WEBHOOK_URL" \
  -H "Content-Type: application/json" \
  -H "goldsky-webhook-secret: $WEBHOOK_SECRET" \
  -d "$PAYLOAD" \
  -v

echo ""
echo ""
echo "✅ Test complete!"