-- Your SQL goes here
CREATE TABLE bridge_events (
    id SERIAL PRIMARY KEY,
    event_id TEXT UNIQUE NOT NULL,
    intent_id TEXT REFERENCES intents(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    chain_id INTEGER NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bridge_events_intent ON bridge_events(intent_id);
CREATE INDEX idx_bridge_events_type ON bridge_events(event_type);
CREATE INDEX idx_bridge_events_chain ON bridge_events(chain_id);
CREATE INDEX idx_bridge_events_tx_hash ON bridge_events(transaction_hash);
CREATE INDEX idx_bridge_events_block ON bridge_events(block_number);
CREATE INDEX idx_bridge_events_nullifier ON bridge_events((event_data->>'nullifier'));
