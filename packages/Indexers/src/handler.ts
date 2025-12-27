import { GoldskyWebhookPayload, RelayerEventPayload, EventType } from './types';
import { getChainName, getContractType, formatEventData } from './utils';

export async function handleIntentCreated(payload: GoldskyWebhookPayload): Promise<RelayerEventPayload> {
  const { event, block, chainId } = payload;
  
  return {
    event_type: EventType.IntentCreated,
    chain: getChainName(chainId),
    chain_id: chainId,
    transaction_hash: event.transactionHash,
    block_number: block.number,
    log_index: event.logIndex,
    contract_address: event.address.toLowerCase(),
    event_data: formatEventData(event.args),
    timestamp: block.timestamp
  };
}

export async function handleIntentRegistered(payload: GoldskyWebhookPayload): Promise<RelayerEventPayload> {
  const { event, block, chainId } = payload;
  
  return {
    event_type: EventType.IntentRegistered,
    chain: getChainName(chainId),
    chain_id: chainId,
    transaction_hash: event.transactionHash,
    block_number: block.number,
    log_index: event.logIndex,
    contract_address: event.address.toLowerCase(),
    event_data: formatEventData(event.args),
    timestamp: block.timestamp
  };
}

export async function handleIntentFilled(payload: GoldskyWebhookPayload): Promise<RelayerEventPayload> {
  const { event, block, chainId } = payload;
  const contractType = getContractType(chainId, event.address);
  
  return {
    event_type: EventType.IntentFilled,
    chain: getChainName(chainId),
    chain_id: chainId,
    transaction_hash: event.transactionHash,
    block_number: block.number,
    log_index: event.logIndex,
    contract_address: event.address.toLowerCase(),
    event_data: {
      ...formatEventData(event.args),
      contract_type: contractType
    },
    timestamp: block.timestamp
  };
}

export async function handleIntentRefunded(payload: GoldskyWebhookPayload): Promise<RelayerEventPayload> {
  const { event, block, chainId } = payload;
  
  return {
    event_type: EventType.IntentRefunded,
    chain: getChainName(chainId),
    chain_id: chainId,
    transaction_hash: event.transactionHash,
    block_number: block.number,
    log_index: event.logIndex,
    contract_address: event.address.toLowerCase(),
    event_data: formatEventData(event.args),
    timestamp: block.timestamp
  };
}

export async function handleWithdrawalClaimed(payload: GoldskyWebhookPayload): Promise<RelayerEventPayload> {
  const { event, block, chainId } = payload;
  
  return {
    event_type: EventType.WithdrawalClaimed,
    chain: getChainName(chainId),
    chain_id: chainId,
    transaction_hash: event.transactionHash,
    block_number: block.number,
    log_index: event.logIndex,
    contract_address: event.address.toLowerCase(),
    event_data: formatEventData(event.args),
    timestamp: block.timestamp
  };
}

export async function handleRootSynced(payload: GoldskyWebhookPayload): Promise<RelayerEventPayload> {
  const { event, block, chainId } = payload;
  
  return {
    event_type: EventType.RootSynced,
    chain: getChainName(chainId),
    chain_id: chainId,
    transaction_hash: event.transactionHash,
    block_number: block.number,
    log_index: event.logIndex,
    contract_address: event.address.toLowerCase(),
    event_data: formatEventData(event.args),
    timestamp: block.timestamp
  };
}