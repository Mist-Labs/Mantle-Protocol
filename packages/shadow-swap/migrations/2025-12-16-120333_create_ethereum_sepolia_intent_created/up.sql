-- Your SQL goes here
CREATE TABLE IF NOT EXISTS ethereum_sepolia_intent_created (
    id SERIAL PRIMARY KEY,
    event_data JSONB NOT NULL,
    block_number BIGINT NOT NULL,
    log_index INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_eth_intent_created_block_log
ON ethereum_sepolia_intent_created (block_number ASC, log_index ASC);

-- Prevent duplicate events (reorg protection)
CREATE UNIQUE INDEX IF NOT EXISTS idx_eth_intent_created_unique
ON ethereum_sepolia_intent_created (block_number, log_index);

CREATE INDEX IF NOT EXISTS idx_eth_intent_created_jsonb
ON ethereum_sepolia_intent_created USING GIN (event_data);
