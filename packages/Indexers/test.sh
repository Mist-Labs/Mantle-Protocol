#!/bin/bash
set -euo pipefail

# ---------- Colors ----------
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# ---------- Load env vars safely if .env exists ----------
if [ -f .env ]; then
  echo -e "${YELLOW}📄 Loading environment variables from .env...${NC}"
  set -a
  while IFS= read -r line; do
    # Skip empty lines and comments
    [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue

    # Strip inline comments
    line=$(echo "$line" | sed 's/[[:space:]]*#.*$//')

    # Skip if empty after stripping
    [[ -z "$line" ]] && continue

    export "$line"
  done < .env
  set +a
  echo -e "${GREEN}✓ Environment variables loaded${NC}"
  echo ""
fi

# ---------- Required vars ----------
: "${WEBHOOK_URL:?WEBHOOK_URL is not set}"
: "${GOLDSKY_WEBHOOK_SECRET:?GOLDSKY_WEBHOOK_SECRET is not set}"

echo "🔗 Webhook configured"
echo "🧪 Testing webhook with Goldsky secret..."
echo "URL: $WEBHOOK_URL"
echo ""

# ---------- Test payload ----------
PAYLOAD='{
  "event_type": "intent_created",
  "intent_id": "0x1234",
  "chain": "ethereum",
  "transaction_hash": "0xabcd",
  "block_number": 12345,
  "timestamp": 1234567890
}'

# ---------- Send request ----------
curl -X POST "$WEBHOOK_URL" \
  -H "Content-Type: application/json" \
  -H "goldsky-webhook-secret: $GOLDSKY_WEBHOOK_SECRET" \
  -d "$PAYLOAD" \
  -v

echo ""
echo -e "${GREEN}✅ Test complete!${NC}"
