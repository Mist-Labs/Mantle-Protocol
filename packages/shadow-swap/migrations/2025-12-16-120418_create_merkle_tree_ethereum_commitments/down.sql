-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_eth_intent_created_jsonb;
DROP INDEX IF EXISTS idx_eth_intent_created_unique;
DROP INDEX IF EXISTS idx_eth_intent_created_block_log;
DROP TABLE IF EXISTS ethereum_sepolia_intent_created;