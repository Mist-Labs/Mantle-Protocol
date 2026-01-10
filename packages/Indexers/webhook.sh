#!/bin/bash
set -euo pipefail

source .env

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
  while IFS= read -r line; do
    [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
    line=$(echo "$line" | sed 's/[[:space:]]*#.*$//')
    [[ -z "$line" ]] && continue
    export "$line"
  done < <(grep -v '^[[:space:]]*$' .env)
  set +a
  echo -e "${GREEN}✓ Environment variables loaded${NC}"
  echo ""
else
  echo -e "${RED}❌ Error: .env file not found!${NC}"
  echo ""
  exit 1
fi

# Validate required environment variables
if [ -z "${WEBHOOK_URL:-}" ] || [ -z "${GOLDSKY_WEBHOOK_SECRET:-}" ]; then
  echo -e "${RED}❌ Error: WEBHOOK_URL or GOLDSKY_WEBHOOK_SECRET is not set${NC}"
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
  fi
  echo ""
}

################################
# Mantle Sepolia Webhooks
################################
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}📡 Creating Mantle Sepolia Webhooks${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

MANTLE_SUBGRAPH="shadowswap-mantle-mantle-sepolia/v3"

# IntentPool Events
echo -e "${YELLOW}IntentPool Events:${NC}"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-created" "intent_created"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-refunded" "intent_refunded"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-settled" "intent_settled"
create_webhook "$MANTLE_SUBGRAPH" "mantle-root-synced" "root_synced"
create_webhook "$MANTLE_SUBGRAPH" "mantle-fill-root-synced" "fill_root_synced"

# Settlement Events
echo -e "${YELLOW}Settlement Events:${NC}"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-registered" "intent_registered"
create_webhook "$MANTLE_SUBGRAPH" "mantle-intent-filled" "intent_filled"
create_webhook "$MANTLE_SUBGRAPH" "mantle-withdrawal-claimed" "withdrawal_claimed"
create_webhook "$MANTLE_SUBGRAPH" "mantle-commitment-root-synced" "commitment_root_synced"
create_webhook "$MANTLE_SUBGRAPH" "mantle-settlement-root-synced" "root_synced"

################################
# Ethereum Sepolia Webhooks
################################
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}📡 Creating Ethereum Sepolia Webhooks${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

ETH_SUBGRAPH="shadowswap-ethereum-sepolia/v3"

# IntentPool Events
echo -e "${YELLOW}IntentPool Events:${NC}"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-created" "intent_created"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-refunded" "intent_refunded"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-settled" "intent_settled"
create_webhook "$ETH_SUBGRAPH" "ethereum-root-synced" "root_synced"
create_webhook "$ETH_SUBGRAPH" "ethereum-fill-root-synced" "fill_root_synced"

# Settlement Events
echo -e "${YELLOW}Settlement Events:${NC}"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-registered" "intent_registered"
create_webhook "$ETH_SUBGRAPH" "ethereum-intent-filled" "intent_filled"
create_webhook "$ETH_SUBGRAPH" "ethereum-withdrawal-claimed" "withdrawal_claimed"
create_webhook "$ETH_SUBGRAPH" "ethereum-commitment-root-synced" "commitment_root_synced"
create_webhook "$ETH_SUBGRAPH" "ethereum-settlement-root-synced" "root_synced"

################################
# Done
################################
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Webhook setup completed!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}📋 Event Summary:${NC}"
echo "  IntentPool: IntentCreated, IntentSettled, IntentRefunded, RootSynced, FillRootSynced"
echo "  Settlement: IntentRegistered, IntentFilled, WithdrawalClaimed, CommitmentRootSynced", "RootSynced"
echo ""