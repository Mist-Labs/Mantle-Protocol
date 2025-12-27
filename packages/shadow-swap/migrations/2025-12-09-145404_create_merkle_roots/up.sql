CREATE TABLE merkle_roots (
    tree_id INTEGER PRIMARY KEY,
    root TEXT NOT NULL,
    leaf_count BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);