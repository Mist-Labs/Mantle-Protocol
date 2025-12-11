-- Your SQL goes here
CREATE TABLE intents (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL,
    amount TEXT NOT NULL,
    dest_chain INTEGER NOT NULL,
    deadline BIGINT NOT NULL,
    source_commitment TEXT,
    dest_fill_txid TEXT,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_intents_status ON intents(status);
CREATE INDEX idx_intents_dest_chain ON intents(dest_chain);
CREATE INDEX idx_intents_created_at ON intents(created_at);
