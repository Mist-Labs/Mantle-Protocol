-- Your SQL goes here
CREATE TABLE chain_transactions (
    id SERIAL PRIMARY KEY,
    intent_id TEXT NOT NULL REFERENCES intents(id) ON DELETE CASCADE,
    chain_id INTEGER NOT NULL,
    tx_type TEXT NOT NULL,
    tx_hash TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_chain_tx_intent ON chain_transactions(intent_id);
CREATE INDEX idx_chain_tx_hash ON chain_transactions(tx_hash);
CREATE INDEX idx_chain_tx_status ON chain_transactions(status);
CREATE INDEX idx_chain_tx_chain_id ON chain_transactions(chain_id);
