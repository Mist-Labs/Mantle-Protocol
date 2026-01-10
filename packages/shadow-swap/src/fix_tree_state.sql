-- ==========================================
-- Tree State Fix Migration Script
-- Run this BEFORE deploying the fixed code
-- ==========================================

BEGIN;

-- 1. Backup current state
CREATE TABLE IF NOT EXISTS merkle_trees_backup AS 
SELECT * FROM merkle_trees;

CREATE TABLE IF NOT EXISTS merkle_nodes_backup AS 
SELECT * FROM merkle_nodes;

-- 2. Clear all Mantle commitment tree data
DELETE FROM merkle_nodes 
WHERE tree_id = (SELECT tree_id FROM merkle_trees WHERE tree_name = 'mantle_commitments');

UPDATE merkle_trees 
SET 
    leaf_count = 0,
    root = '0x0000000000000000000000000000000000000000000000000000000000000000',
    updated_at = NOW()
WHERE tree_name = 'mantle_commitments';

-- 3. Clear all Mantle intents tree data
DELETE FROM merkle_nodes 
WHERE tree_id = (SELECT tree_id FROM merkle_trees WHERE tree_name = 'mantle_intents');

UPDATE merkle_trees 
SET 
    leaf_count = 0,
    root = '0x0000000000000000000000000000000000000000000000000000000000000000',
    updated_at = NOW()
WHERE tree_name = 'mantle_intents';

-- 4. Clear all Ethereum commitment tree data
DELETE FROM merkle_nodes 
WHERE tree_id = (SELECT tree_id FROM merkle_trees WHERE tree_name = 'ethereum_commitments');

UPDATE merkle_trees 
SET 
    leaf_count = 0,
    root = '0x0000000000000000000000000000000000000000000000000000000000000000',
    updated_at = NOW()
WHERE tree_name = 'ethereum_commitments';

-- 5. Clear all Ethereum intents tree data
DELETE FROM merkle_nodes 
WHERE tree_id = (SELECT tree_id FROM merkle_trees WHERE tree_name = 'ethereum_intents');

UPDATE merkle_trees 
SET 
    leaf_count = 0,
    root = '0x0000000000000000000000000000000000000000000000000000000000000000',
    updated_at = NOW()
WHERE tree_name = 'ethereum_intents';

-- 6. Verify current state
SELECT 
    tree_name,
    leaf_count,
    LEFT(root, 20) as root_prefix,
    (SELECT COUNT(*) FROM intents 
     WHERE source_chain = CASE 
         WHEN tree_name LIKE '%mantle%' THEN 'mantle'
         ELSE 'ethereum'
     END
     AND source_commitment IS NOT NULL
     AND block_number IS NOT NULL
     AND log_index IS NOT NULL) as db_commitments
FROM merkle_trees
WHERE tree_name IN ('mantle_commitments', 'ethereum_commitments', 'mantle_intents', 'ethereum_intents')
ORDER BY tree_name;

-- 7. Check for any problematic intents (NULL block_number or log_index)
SELECT 
    source_chain,
    COUNT(*) as count,
    COUNT(*) FILTER (WHERE block_number IS NULL) as null_block_number,
    COUNT(*) FILTER (WHERE log_index IS NULL) as null_log_index
FROM intents
WHERE source_commitment IS NOT NULL
GROUP BY source_chain;

COMMIT;

-- ==========================================
-- Post-migration verification queries
-- ==========================================

-- All trees should now have leaf_count = 0
SELECT tree_name, leaf_count FROM merkle_trees 
WHERE tree_name LIKE '%commitment%' OR tree_name LIKE '%intent%';

-- This shows how many commitments will be added when app starts
SELECT 
    source_chain,
    COUNT(*) as commitments_to_add
FROM intents
WHERE source_commitment IS NOT NULL
  AND block_number IS NOT NULL
  AND log_index IS NOT NULL
GROUP BY source_chain;