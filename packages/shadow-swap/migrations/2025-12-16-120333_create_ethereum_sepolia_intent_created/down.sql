-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_ethereum_commitments_commitment;
DROP INDEX IF EXISTS idx_unique_ethereum_commitment;
DROP INDEX IF EXISTS idx_ethereum_commitments_created;
DROP TABLE IF EXISTS merkle_tree_ethereum_commitments;
