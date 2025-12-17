-- Your SQL goes here
CREATE TABLE IF NOT EXISTS root_syncs (
    id SERIAL PRIMARY KEY,
    sync_type TEXT NOT NULL,
    root TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);


CREATE INDEX IF NOT EXISTS idx_root_syncs_sync_type ON root_syncs(sync_type);
CREATE INDEX IF NOT EXISTS idx_root_syncs_created_at ON root_syncs(created_at DESC);
