CREATE TABLE merkle_roots (
    tree_id TEXT PRIMARY KEY,
    root_hash TEXT NOT NULL,
    leaf_count BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);