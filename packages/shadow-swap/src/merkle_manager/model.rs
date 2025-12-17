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
