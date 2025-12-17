-- Your SQL goes here
CREATE TABLE IF NOT EXISTS merkle_tree_ethereum_commitments (
    id SERIAL PRIMARY KEY,
    commitment TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ethereum_commitments_created
ON merkle_tree_ethereum_commitments (created_at ASC);

-- Prevent duplicate commitments (critical for merkle tree integrity)
CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_ethereum_commitment
ON merkle_tree_ethereum_commitments (commitment);

CREATE INDEX IF NOT EXISTS idx_ethereum_commitments_commitment
ON merkle_tree_ethereum_commitments (commitment);