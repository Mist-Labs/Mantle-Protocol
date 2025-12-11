use chrono::Utc;

// DB: merkle_nodes
pub struct DbMerkleNode {
    pub tree_id: String,      // e.g., "mantle", "ethereum"
    pub node_index: i64,      // zero-based index in complete binary tree
    pub hash: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

// DB: merkle_roots
pub struct DbMerkleRoot {
    pub tree_id: String,
    pub root_hash: String,
    pub leaf_count: i64,
    pub updated_at: chrono::DateTime<Utc>,
}

pub struct MerkleProof {
    pub path: Vec<String>,
    pub leaf_index: usize,
    pub root: String,
}

impl MerkleProof {
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.path.len()
    }
}
