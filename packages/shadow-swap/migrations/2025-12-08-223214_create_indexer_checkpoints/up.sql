-- Your SQL goes here
CREATE TABLE indexer_checkpoints (
    chain TEXT PRIMARY KEY,
    last_block INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_indexer_chain ON indexer_checkpoints(chain);
