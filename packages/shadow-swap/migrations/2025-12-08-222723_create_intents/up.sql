-- Your SQL goes here
CREATE TABLE intents (
    id TEXT PRIMARY KEY,
    user_address VARCHAR(42) NOT NULL,
    source_chain VARCHAR(50) NOT NULL,
    dest_chain VARCHAR(50) NOT NULL,
    source_token TEXT NOT NULL,
    dest_token VARCHAR(42) NOT NULL,
    amount TEXT NOT NULL,
    dest_amount VARCHAR(78) NOT NULL,
    source_commitment TEXT,
    dest_fill_txid TEXT,
    dest_registration_txid VARCHAR(66),
    source_complete_txid VARCHAR(66),
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deadline BIGINT NOT NULL,
    refund_address VARCHAR(42)
);

CREATE INDEX idx_intents_status ON intents(status);
CREATE INDEX idx_intents_dest_chain ON intents(dest_chain);
CREATE INDEX idx_intents_created_at ON intents(created_at);