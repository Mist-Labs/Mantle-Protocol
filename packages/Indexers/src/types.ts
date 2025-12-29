export interface GoldskyWebhookPayload {
  data: {
    new: {
      block$: number;
      block_number: string;
      chain_id: string;
      contract_id: string;
      id: string;
      transaction_hash: string;
      timestamp: string;
      vid: string;
      [key: string]: any;
    };
    old: any | null;
  };
  data_source: string;
  entity: string; 
  id: string;
  op: "INSERT" | "UPDATE" | "DELETE";
  webhook_id: string;
  webhook_name: string;
}

export interface RelayerEventPayload {
  event_type: string;
  chain: string;
  chain_id: number;
  transaction_hash: string;
  block_number: number;
  log_index?: number;
  contract_address: string;
  event_data: Record<string, any>;
  timestamp: number;
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

// Map Goldsky entity names to our event types
export const ENTITY_TO_EVENT_TYPE: Record<string, EventType> = {
  root_synced: EventType.RootSynced,
  intent_created: EventType.IntentCreated,
  intent_registered: EventType.IntentRegistered,
  intent_filled: EventType.IntentFilled,
  intent_marked_filled: EventType.IntentMarkedFilled,
  intent_refunded: EventType.IntentRefunded,
  withdrawal_claimed: EventType.WithdrawalClaimed,
};


export enum Chain {
  Mantle = "mantle",
  Ethereum = "ethereum",
}
