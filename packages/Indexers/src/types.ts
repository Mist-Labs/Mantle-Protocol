export interface GoldskyWebhookPayload {
  webhookId: string;
  webhookName: string;
  chainId: number;
  event: {
    name: string;
    logIndex: number;
    transactionHash: string;
    transactionIndex: number;
    address: string;
    blockHash: string;
    blockNumber: number;
    data: string;
    topics: string[];
    args: Record<string, any>;
  };
  block: {
    hash: string;
    number: number;
    timestamp: number;
  };
}

export interface RelayerEventPayload {
  event_type: string;
  chain: string;
  chain_id: number;
  transaction_hash: string;
  block_number: number;
  log_index: number;
  contract_address: string;
  event_data: Record<string, any>;
  timestamp: number;
}

export interface EventHandler {
  (payload: GoldskyWebhookPayload): Promise<RelayerEventPayload | null>;
}

export enum EventType {
  IntentCreated = "intent_created",
  IntentRegistered = "intent_registered",
  IntentFilled = "intent_filled",
  IntentMarkedFilled = "intent_marked_filled",
  IntentRefunded = "intent_refunded",
  WithdrawalClaimed = "withdrawal_claimed",
  RootSynced = "root_synced",
}

export enum Chain {
  Mantle = "mantle",
  Ethereum = "ethereum",
}
