import { GoldskyWebhookPayload, RelayerEventPayload, EventType } from "./types";

function getChainName(chainId: string): string {
  switch (chainId) {
    case "5003":
      return "mantle";
    case "11155111":
      return "ethereum";
    default:
      return "unknown";
  }
}

function parseEventDataFromEntity(
  eventData: any,
  entity: string
): Record<string, any> {
  // Map Goldsky database fields to event data structure
  const baseData: Record<string, any> = {
    intentId: eventData.intent_id || eventData.id.split("-")[0],
    transactionHash: eventData.transaction_hash,
    blockNumber: eventData.block_number,
  };

  // Entity-specific field mappings
  switch (entity) {
    case "intent_created":
      return {
        ...baseData,
        commitment: eventData.commitment,
        token: eventData.token,
        amount: eventData.amount,
        destChain: eventData.dest_chain,
      };

    case "intent_registered":
      return {
        ...baseData,
        commitment: eventData.commitment,
        recipient: eventData.recipient,
      };

    case "intent_filled":
      return {
        ...baseData,
        solver: eventData.solver || eventData.filler,
        token: eventData.token,
        amount: eventData.amount,
      };

    case "intent_marked_filled":
      return {
        ...baseData,
        solver: eventData.solver,
        fillRoot: eventData.fill_root || eventData.root,
      };

    case "intent_refunded":
      return {
        ...baseData,
        amount: eventData.amount,
      };

    case "withdrawal_claimed":
      return {
        ...baseData,
        nullifier: eventData.nullifier,
        recipient: eventData.recipient,
        amount: eventData.amount,
      };

    case "root_synced":
      return {
        root: eventData.root,
        chainId: eventData.source_chain_id || eventData.chain_id,
      };

    default:
      return baseData;
  }
}

export async function transformGoldskyPayload(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  const { entity, data } = payload;
  const eventData = data.new;

  const eventType = entity as EventType;
  const chainId = parseInt(eventData.chain_id);

  return {
    event_type: eventType,
    chain: getChainName(eventData.chain_id),
    chain_id: chainId,
    transaction_hash: eventData.transaction_hash,
    block_number: parseInt(eventData.block_number),
    log_index: parseInt(eventData.id.split("-")[1] || "0"),
    contract_address: eventData.contract_id.toLowerCase(),
    event_data: parseEventDataFromEntity(eventData, entity),
    timestamp: parseInt(eventData.timestamp),
  };
}

// Individual handlers remain but now use transformed payload
export async function handleIntentCreated(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}

export async function handleIntentRegistered(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}

export async function handleIntentFilled(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}

export async function handleIntentMarkedFilled(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}

export async function handleIntentRefunded(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}

export async function handleWithdrawalClaimed(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}

export async function handleRootSynced(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}
