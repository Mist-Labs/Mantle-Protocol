import {
  GoldskyWebhookPayload,
  RelayerEventPayload,
  EventType,
  Chain,
} from "./types";
import { deriveChainId, getChainName } from "./utils";

function getChainNameString(chainId: string): string {
  try {
    const chain = getChainName(chainId);
    return chain === Chain.Mantle ? "mantle" : "ethereum";
  } catch {
    return "unknown";
  }
}

function parseEventDataFromEntity(
  eventData: any,
  entity: string
): Record<string, any> {
  const baseData: Record<string, any> = {
    transactionHash: eventData.transaction_hash,
    blockNumber: eventData.block_number,
  };

  switch (entity) {
    case "intent_created":
      return {
        ...baseData,
        intentId: eventData.intent_id || eventData.id.split("-")[0],
        commitment: eventData.commitment,
        sourceToken: eventData.source_token,
        sourceAmount: eventData.source_amount,
        destToken: eventData.dest_token,
        destAmount: eventData.dest_amount,
        destChain: eventData.dest_chain || eventData.destChain,
      };

    case "intent_registered":
      return {
        ...baseData,
        intentId: eventData.intent_id || eventData.id.split("-")[0],
        commitment: eventData.commitment,
        recipient: eventData.recipient,
      };

    case "intent_filled":
      return {
        ...baseData,
        intentId: eventData.intent_id || eventData.id.split("-")[0],
        solver: eventData.solver || eventData.filler,
        token: eventData.token,
        amount: eventData.amount,
      };

    case "intent_settled":
      return {
        ...baseData,
        intentId: eventData.intent_id || eventData.id.split("-")[0],
        solver: eventData.solver,
        fillRoot: eventData.fill_root || eventData.root,
      };

    case "intent_refunded":
      return {
        ...baseData,
        intentId: eventData.intent_id || eventData.id.split("-")[0],
        amount: eventData.amount,
      };

    case "withdrawal_claimed":
      return {
        ...baseData,
        intentId: eventData.intent_id || eventData.id.split("-")[0],
        nullifier: eventData.nullifier,
        recipient: eventData.recipient,
        amount: eventData.amount,
      };

    case "fill_root_synced":
      return {
        ...baseData,
        root: eventData.root,
        chainId: eventData.chain_id || eventData.chainId,
        type: "FILL",
      };

    case "commitment_root_synced":
      return {
        ...baseData,
        root: eventData.root,
        chainId: eventData.chain_id || eventData.chainId,
        type: "COMMITMENT",
      };

    case "root_synced":
      return {
        ...baseData,
        root: eventData.root,
        chainId:
          eventData.source_chain_id ||
          eventData.dest_chain_id ||
          eventData.chain_id,
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
  const chainId = deriveChainId(payload);

  return {
    event_type: eventType,
    chain: getChainNameString(chainId),
    chain_id: parseInt(chainId),
    transaction_hash: eventData.transaction_hash,
    block_number: parseInt(eventData.block_number),
    log_index: parseInt(eventData.id.split("-")[1] || "0"),
    contract_address: eventData.contract_id.toLowerCase(),
    event_data: parseEventDataFromEntity(eventData, entity),
    timestamp: parseInt(eventData.timestamp),
  };
}

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

export async function handleIntentSettled(
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

export async function handleFillRootSynced(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}

export async function handleCommitmentRootSynced(
  payload: GoldskyWebhookPayload
): Promise<RelayerEventPayload> {
  return transformGoldskyPayload(payload);
}
