import crypto from "crypto";
import { config } from "./config";
import { Chain, GoldskyWebhookPayload } from "./types";

// Contract address to chain ID mapping for Goldsky events
export const CONTRACT_CHAIN_MAP: Record<string, string> = {
  // Ethereum Sepolia contracts
  "0xcb46d916522D7c6853fcE2aa5F337e0a3626E263": "11155111", // ETHEREUM_INTENT_POOL_ADDRESS
  "0x7CCC9864125143e6c530506772Eaf5595DC14897": "11155111", // ETHEREUM_SETTLEMENT_ADDRESS

  // Mantle Sepolia contracts
  "0x6ebcF830b855108Fa44AbED6Ba964F2Af9C34424": "5003", // MANTLE_INTENT_POOL_ADDRESS
  "0x1c4F9eBeccE31cEFe2FDe415b05184b4ea46908f": "5003", // MANTLE_SETTLEMENT_ADDRESS
};

export function createHmacSignature(payload: string): string {
  return crypto
    .createHmac("sha256", config.hmacSecret)
    .update(payload)
    .digest("hex");
}

export function getChainName(chainId: number | string): Chain {
  const id = typeof chainId === "string" ? parseInt(chainId) : chainId;

  if (id === config.chains.mantle.chainId || id === 5003) return Chain.Mantle;
  if (id === config.chains.ethereum.chainId || id === 11155111)
    return Chain.Ethereum;
  throw new Error(`Unknown chain ID: ${chainId}`);
}

export function getContractType(
  chainId: number,
  address: string
): "intent_pool" | "settlement" {
  const normalizedAddress = address.toLowerCase();
  const chain = getChainName(chainId);

  if (chain === Chain.Mantle) {
    if (normalizedAddress === config.chains.mantle.intentPoolAddress)
      return "intent_pool";
    if (normalizedAddress === config.chains.mantle.settlementAddress)
      return "settlement";
  } else {
    if (normalizedAddress === config.chains.ethereum.intentPoolAddress)
      return "intent_pool";
    if (normalizedAddress === config.chains.ethereum.settlementAddress)
      return "settlement";
  }

  throw new Error(`Unknown contract address: ${address} on chain ${chainId}`);
}

export function formatEventData(
  args: Record<string, any>
): Record<string, any> {
  const formatted: Record<string, any> = {};

  for (const [key, value] of Object.entries(args)) {
    if (typeof value === "bigint") {
      formatted[key] = value.toString();
    } else if (
      typeof value === "object" &&
      value !== null &&
      "type" in value &&
      value.type === "BigNumber"
    ) {
      formatted[key] = value.hex || value.toString();
    } else {
      formatted[key] = value;
    }
  }

  return formatted;
}

/**
 * Derives chain ID from Goldsky webhook payload
 * Used for events that don't have chain_id in their data
 */
export function deriveChainId(payload: GoldskyWebhookPayload): string {
  const eventData = payload.data.new;

  // First check if chain_id exists directly in event data
  if (eventData.chain_id) {
    return eventData.chain_id;
  }

  // Use contract address mapping (most reliable method)
  const contractAddress = eventData.contract_id?.toLowerCase();
  if (contractAddress && CONTRACT_CHAIN_MAP[contractAddress]) {
    return CONTRACT_CHAIN_MAP[contractAddress];
  }

  // Fallback: derive from data_source or webhook_name
  const dataSource = payload.data_source.toLowerCase();
  const webhookName = payload.webhook_name?.toLowerCase() || "";

  if (dataSource.includes("mantle") || webhookName.includes("mantle")) {
    return "5003";
  }

  if (
    dataSource.includes("ethereum") ||
    dataSource.includes("sepolia") ||
    webhookName.includes("ethereum")
  ) {
    return "11155111";
  }

  return ""; // Unknown
}
