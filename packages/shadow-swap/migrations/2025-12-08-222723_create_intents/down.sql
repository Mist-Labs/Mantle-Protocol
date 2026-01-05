-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_intents_solver_status;
DROP INDEX IF EXISTS idx_intents_solver_address;
DROP COLUMN IF EXISTS solver_address;
DROP TABLE IF EXISTS intents;
