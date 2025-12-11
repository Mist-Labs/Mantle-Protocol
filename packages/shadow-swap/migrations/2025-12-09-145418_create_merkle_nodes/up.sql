CREATE TABLE merkle_nodes (
    node_id SERIAL PRIMARY KEY,
    tree_id INT NOT NULL REFERENCES merkle_trees(tree_id) ON DELETE CASCADE,
    level INT NOT NULL,
    node_index BIGINT NOT NULL,
    hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tree_id, level, node_index)
);