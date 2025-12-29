#!/bin/bash
set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🔧 Goldsky Webhook Setup${NC}"
echo ""

# Load env vars safely if .env exists
if [ -f .env ]; then
  echo -e "${YELLOW}📄 Loading environment variables from .env...${NC}"
  set -a
  # More robust .env parsing that handles comments and whitespace
  # This strips inline comments, empty lines, and leading/trailing whitespace
  while IFS= read -r line; do
    # Skip empty lines and lines starting with #
    [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
    # Remove inline comments (anything after # that's not in quotes)
    line=$(echo "$line" | sed 's/[[:space:]]*#.*$//')
    # Skip if line becomes empty after removing comment
    [[ -z "$line" ]] && continue
    # Export the variable
    export "$line"
  done < <(grep -v '^[[:space:]]*$' .env)
  set +a
  echo -e "${GREEN}✓ Environment variables loaded${NC}"
  echo ""
else
  echo -e "${RED}❌ Error: .env file not found!${NC}"
  echo ""
  echo "Please create a .env file with the following variables:"
  echo "  WEBHOOK_URL=https://your-backend-url.com/webhook"
  echo "  GOLDSKY_WEBHOOK_SECRET=your-secret-key"
  echo ""
  exit 1
fi

# Validate required environment variables
if [ -z "${WEBHOOK_URL:-}" ]; then
  echo -e "${RED}❌ Error: WEBHOOK_URL is not set in .env file${NC}"
  echo ""
  echo "Please add to your .env file:"
  echo "  WEBHOOK_URL=https://your-backend-url.com/webhook"
  echo ""
  exit 1
fi

if [ -z "${GOLDSKY_WEBHOOK_SECRET:-}" ]; then
  echo -e "${RED}❌ Error: GOLDSKY_WEBHOOK_SECRET is not set in .env file${NC}"
  echo ""
  echo "Please add to your .env file:"
  echo "  GOLDSKY_WEBHOOK_SECRET=your-secret-key"
  echo ""
  exit 1
fi

echo -e "${GREEN}✓ Environment validation passed${NC}"
echo -e "${BLUE}Webhook URL: ${WEBHOOK_URL}${NC}"
echo ""

# Function to create webhook with error handling
create_webhook() {
  local subgraph=$1
  local name=$2
  local entity=$3
  
  echo -e "${YELLOW}  Creating: ${name}...${NC}"
  
  if goldsky subgraph webhook create "$subgraph" \
    --name "$name" \
    --entity "$entity" \
    --url "$WEBHOOK_URL" \
    --secret "$GOLDSKY_WEBHOOK_SECRET" 2>&1; then
    echo -e "${GREEN}  ✓ ${name} created${NC}"
  else
    echo -e "${RED}  ✗ Failed to create ${name}${NC}"
    echo -e "${YELLOW}  (webhook may already exist or entity '${entity}' doesn't exist)${NC}"
    echo -e "${YELLOW}  Run: goldsky subgraph webhook list-entities ${subgraph}${NC}"
  fi
  echo ""
}

################################
# Mantle Sepolia Webhooks
# Contract: PrivateIntentPool + PrivateSettlement
################################
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}📡 Creating Mantle Sepolia Webhooks${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

MANTLE_SUBGRAPH="shadowswap-mantle-mantle-sepolia/v2"

# IntentPool Events
echo -e "${YELLOW}IntentPool Events:${NC}"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-created" "intent_created"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-refunded" "intent_refunded"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-marked-filled" "intent_marked_filled"
create_webhook "$MANTLE_SUBGRAPH" "mantle-root-synced" "root_synced"

# Settlement Events
echo -e "${YELLOW}Settlement Events:${NC}"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-registered" "intent_registered"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-filled" "intent_filled"
create_webhook "$MANTLE_SUBGRAPH" "mantle-withdrawal-claimed" "withdrawal_claimed"

################################
# Ethereum Sepolia Webhooks
# Contract: PrivateIntentPool + PrivateSettlement
################################
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}📡 Creating Ethereum Sepolia Webhooks${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

ETH_SUBGRAPH="shadowswap-ethereum-sepolia/v2"

# IntentPool Events
echo -e "${YELLOW}IntentPool Events:${NC}"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-created" "intent_created"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-refunded" "intent_refunded"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-marked-filled" "intent_marked_filled"
create_webhook "$ETH_SUBGRAPH" "ethereum-root-synced" "root_synced"

# Settlement Events
echo -e "${YELLOW}Settlement Events:${NC}"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-registered" "intent_registered"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-filled" "intent_filled"
create_webhook "$ETH_SUBGRAPH" "ethereum-withdrawal-claimed" "withdrawal_claimed"

################################
# Done
################################
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Webhook setup completed!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}📋 Event Summary:${NC}"
echo "  IntentPool: IntentCreated, IntentMarkedFilled, IntentRefunded, RootSynced"
echo "  Settlement: IntentRegistered, IntentFilled, WithdrawalClaimed"
echo ""
echo -e "${BLUE}📝 Useful Commands:${NC}"
echo ""
echo "  List available entities (Mantle):"
echo -e "    ${YELLOW}goldsky subgraph webhook list-entities ${MANTLE_SUBGRAPH}${NC}"
echo ""
echo "  List available entities (Ethereum):"
echo -e "    ${YELLOW}goldsky subgraph webhook list-entities ${ETH_SUBGRAPH}${NC}"
echo ""
echo "  List Mantle webhooks:"
echo -e "    ${YELLOW}goldsky subgraph webhook list ${MANTLE_SUBGRAPH}${NC}"
echo ""
echo "  List Ethereum webhooks:"
echo -e "    ${YELLOW}goldsky subgraph webhook list ${ETH_SUBGRAPH}${NC}"
echo ""
echo "  Delete a webhook:"
echo -e "    ${YELLOW}goldsky subgraph webhook delete ${MANTLE_SUBGRAPH} --name <webhook-name>${NC}"
echo ""
echo "  Test webhook endpoint:"
echo -e "    ${YELLOW}curl -X POST ${WEBHOOK_URL} \\${NC}"
echo -e "    ${YELLOW}  -H 'Content-Type: application/json' \\${NC}"
echo -e "    ${YELLOW}  -H 'goldsky-webhook-secret: ${GOLDSKY_WEBHOOK_SECRET}' \\${NC}"
echo -e "    ${YELLOW}  -d '{\"test\":true}'${NC}"
echo ""