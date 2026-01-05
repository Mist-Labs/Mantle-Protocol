use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    database::database::Database,
    merkle_manager::model::MerkleProof,
    relay_coordinator::model::{EthereumRelayer, MantleRelayer},
};

const ZERO_LEAF: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
const MANTLE_CHAIN_ID: u32 = 5003;
const ETHEREUM_CHAIN_ID: u32 = 11155111;

pub struct MerkleTreeManager {
    mantle_relayer: Arc<MantleRelayer>,
    ethereum_relayer: Arc<EthereumRelayer>,
    database: Arc<Database>,
    tree_depth: usize,
}

impl MerkleTreeManager {
    pub fn new(
        mantle_relayer: Arc<MantleRelayer>,
        ethereum_relayer: Arc<EthereumRelayer>,
        database: Arc<Database>,
        tree_depth: usize,
    ) -> Self {
        Self {
            mantle_relayer,
            ethereum_relayer,
            database,
            tree_depth,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("🌳 Merkle Tree Manager starting");

        for (tree_name, depth) in &[
            ("mantle_intents", 20),
            ("ethereum_fills", 20),
            ("ethereum_commitments", 20),
        ] {
            self.database.ensure_merkle_tree(tree_name, *depth)?;
        }

        loop {
            if let Err(e) = self.rebuild_mantle_tree().await {
                error!("Failed to rebuild mantle tree: {}", e);
            }

            if let Err(e) = self.rebuild_ethereum_tree().await {
                error!("Failed to rebuild ethereum tree: {}", e);
            }

            if let Err(e) = self.rebuild_ethereum_commitment_tree().await {
                error!("Failed to rebuild ethereum commitment tree: {}", e);
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    pub async fn append_commitment(&self, commitment: &str, chain_id: u32) -> Result<usize> {
        match chain_id {
            MANTLE_CHAIN_ID => self.append_mantle_leaf(commitment).await,
            ETHEREUM_CHAIN_ID => self.append_ethereum_leaf(commitment).await,
            _ => Err(anyhow::anyhow!("Unsupported chain_id: {}", chain_id)),
        }
    }

    pub async fn append_mantle_leaf(&self, commitment: &str) -> Result<usize> {
        let size = self.database.get_mantle_tree_size()?;
        let index = size;

        self.database.add_to_mantle_tree(commitment)?;
        self.database.set_mantle_node(0, index, commitment)?;

        let mut curr_index = index;
        let mut curr_hash = commitment.to_string();

        for level in 0..self.tree_depth {
            let sibling_index = if curr_index % 2 == 0 {
                curr_index + 1
            } else {
                curr_index - 1
            };

            let sibling = self
                .database
                .get_mantle_node(level, sibling_index)?
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;

            let parent_index = curr_index / 2;
            self.database
                .set_mantle_node(level + 1, parent_index, &parent_hash)?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database.record_root("mantle", &curr_hash)?;
        info!("✅ Mantle root: {}", curr_hash);

        Ok(index)
    }

    pub async fn append_ethereum_leaf(&self, intent_id: &str) -> Result<usize> {
        let size = self.database.get_ethereum_tree_size()?;
        let index = size;

        self.database.add_to_ethereum_tree(intent_id)?;
        self.database.set_ethereum_node(0, index, intent_id)?;

        let mut curr_index = index;
        let mut curr_hash = intent_id.to_string();

        for level in 0..self.tree_depth {
            let sibling_index = if curr_index % 2 == 0 {
                curr_index + 1
            } else {
                curr_index - 1
            };

            let sibling = self
                .database
                .get_ethereum_node(level, sibling_index)?
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;

            let parent_index = curr_index / 2;
            self.database
                .set_ethereum_node(level + 1, parent_index, &parent_hash)?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database.record_root("ethereum", &curr_hash)?;
        info!("✅ Ethereum root: {}", curr_hash);

        Ok(index)
    }

    pub async fn rebuild_mantle_tree(&self) -> Result<()> {
        info!("🔨 Rebuilding Mantle tree");

        let mut events = self.database.get_all_mantle_intents()?;
        if events.is_empty() {
            info!("✅ No Mantle intents");
            return Ok(());
        }

        events.sort_by_key(|e| (e.block_number, e.log_index));

        self.database.clear_mantle_tree()?;
        self.database.clear_mantle_nodes()?;

        for event in events {
            self.append_mantle_leaf(&event.commitment).await?;
        }

        let root = self.compute_mantle_commitment_root()?;
        info!("✅ Mantle tree rebuilt: {}", root);

        Ok(())
    }

    pub async fn rebuild_ethereum_tree(&self) -> Result<()> {
        info!("🔨 Rebuilding Ethereum fill tree");

        let mut events = self.database.get_all_ethereum_fills()?;
        if events.is_empty() {
            info!("✅ No Ethereum fills");
            return Ok(());
        }

        events.sort_by_key(|e| (e.block_number, e.log_index));

        self.database.clear_ethereum_tree()?;
        self.database.clear_ethereum_nodes()?;

        for event in events {
            self.append_ethereum_leaf(&event.intent_id).await?;
        }

        let root = self.compute_ethereum_root()?;
        info!("✅ Ethereum fill tree rebuilt: {}", root);

        Ok(())
    }

    pub async fn rebuild_ethereum_commitment_tree(&self) -> Result<()> {
        info!("🔨 Rebuilding Ethereum commitment tree");

        let commitments = self.database.get_all_ethereum_commitments()?;
        if commitments.is_empty() {
            info!("✅ No Ethereum commitments");

            self.database
                .record_root("ethereum_commitments", ZERO_LEAF)?;
            return Ok(());
        }

        self.database.clear_ethereum_commitment_tree()?;
        self.database.clear_ethereum_commitment_nodes()?;

        for commitment in commitments {
            self.append_ethereum_commitment_leaf(&commitment).await?;
        }

        let root = self.compute_ethereum_commitment_root()?;
        info!("✅ Ethereum commitment tree rebuilt: {}", root);

        Ok(())
    }

    pub async fn append_ethereum_commitment_leaf(&self, commitment: &str) -> Result<usize> {
        let size = self.database.get_ethereum_commitment_tree_size()?;
        let index = size;

        self.database.add_to_ethereum_commitment_tree(commitment)?;
        self.database
            .set_ethereum_commitment_node(0, index, commitment)?;

        let mut curr_index = index;
        let mut curr_hash = commitment.to_string();

        for level in 0..self.tree_depth {
            let sibling_index = if curr_index % 2 == 0 {
                curr_index + 1
            } else {
                curr_index - 1
            };

            let sibling = self
                .database
                .get_ethereum_commitment_node(level, sibling_index)?
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;

            let parent_index = curr_index / 2;
            self.database
                .set_ethereum_commitment_node(level + 1, parent_index, &parent_hash)?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database
            .record_root("ethereum_commitments", &curr_hash)?;
        info!("✅ Ethereum commitment root: {}", curr_hash);

        Ok(index)
    }

    pub fn compute_ethereum_commitment_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("ethereum_commitments") {
            return Ok(root);
        }

        let tree = self.database.get_all_ethereum_commitments()?;

        if tree.is_empty() {
            self.database
                .record_root("ethereum_commitments", ZERO_LEAF)?;
            return Ok(ZERO_LEAF.to_string());
        }

        let root = self.compute_root_from_leaves(&tree)?;

        self.database.record_root("ethereum_commitments", &root)?;

        Ok(root)
    }

    pub fn compute_mantle_commitment_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("mantle") {
            return Ok(root);
        }

        let tree = self.database.get_mantle_tree()?;

        if tree.is_empty() {
            self.database.record_root("mantle", ZERO_LEAF)?;
            return Ok(ZERO_LEAF.to_string());
        }

        let root = self.compute_root_from_leaves(&tree)?;
        self.database.record_root("mantle", &root)?;

        Ok(root)
    }

    fn compute_ethereum_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("ethereum") {
            return Ok(root);
        }

        let tree = self.database.get_ethereum_tree()?;

        if tree.is_empty() {
            self.database.record_root("ethereum", ZERO_LEAF)?;
            return Ok(ZERO_LEAF.to_string());
        }

        let root = self.compute_root_from_leaves(&tree)?;
        self.database.record_root("ethereum", &root)?;

        Ok(root)
    }

    // ============================================================
    // PROOF GENERATION METHODS
    // ============================================================

    pub async fn generate_mantle_proof(&self, commitment: &str) -> Result<MerkleProof> {
        let tree = self.database.get_mantle_tree()?;
        let index = tree
            .iter()
            .position(|c| c == commitment)
            .ok_or_else(|| anyhow::anyhow!("Commitment not found"))?;

        let proof = self.compute_merkle_proof(&tree, index)?;
        let root = self.compute_mantle_commitment_root()?;

        Ok(MerkleProof {
            path: proof,
            leaf_index: index,
            root,
        })
    }

    // ============================================================
    // ROOT COMPUTATION HELPERS
    // ============================================================
    pub async fn generate_ethereum_commitment_proof(
        &self,
        commitment: &str,
    ) -> Result<MerkleProof> {
        let tree = self.database.get_ethereum_commitment_tree()?;
        let index = tree
            .iter()
            .position(|c| c == commitment)
            .ok_or_else(|| anyhow!("Commitment not found"))?;
        let proof = self.compute_merkle_proof(&tree, index)?;
        let root = self.compute_ethereum_commitment_root()?;

        Ok(MerkleProof {
            path: proof,
            leaf_index: index,
            root,
        })
    }

    fn compute_root_from_leaves(&self, leaves: &[String]) -> Result<String> {
        if leaves.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        use std::collections::HashMap;

        let mut nodes: HashMap<(usize, usize), String> = HashMap::new();

        for (idx, leaf) in leaves.iter().enumerate() {
            nodes.insert((0, idx), leaf.clone());
        }

        for leaf_idx in 0..leaves.len() {
            let mut curr_index = leaf_idx;
            let mut curr_hash = leaves[leaf_idx].clone();

            for level in 0..self.tree_depth {
                let sibling_index = if curr_index % 2 == 0 {
                    curr_index + 1
                } else {
                    curr_index - 1
                };

                // Get sibling (either from nodes or use ZERO_LEAF)
                let sibling = nodes
                    .get(&(level, sibling_index))
                    .cloned()
                    .unwrap_or_else(|| ZERO_LEAF.to_string());

                let parent_hash = self.hash_pair(&curr_hash, &sibling)?;
                let parent_index = curr_index / 2;

                nodes.insert((level + 1, parent_index), parent_hash.clone());

                curr_index = parent_index;
                curr_hash = parent_hash;
            }
        }

        Ok(nodes
            .get(&(self.tree_depth, 0))
            .cloned()
            .unwrap_or_else(|| ZERO_LEAF.to_string()))
    }

    fn compute_merkle_proof(&self, leaves: &[String], index: usize) -> Result<Vec<String>> {
        if index >= leaves.len() {
            return Err(anyhow::anyhow!("Index out of bounds"));
        }

        use std::collections::HashMap;
        let mut nodes: HashMap<(usize, usize), String> = HashMap::new();

        // Initialize level 0
        for (idx, leaf) in leaves.iter().enumerate() {
            nodes.insert((0, idx), leaf.clone());
        }

        // Build tree for all leaves
        for leaf_idx in 0..leaves.len() {
            let mut curr_index = leaf_idx;
            let mut curr_hash = leaves[leaf_idx].clone();

            for level in 0..self.tree_depth {
                let sibling_index = if curr_index % 2 == 0 {
                    curr_index + 1
                } else {
                    curr_index - 1
                };

                let sibling = nodes
                    .get(&(level, sibling_index))
                    .cloned()
                    .unwrap_or_else(|| ZERO_LEAF.to_string());

                let parent_hash = self.hash_pair(&curr_hash, &sibling)?;
                let parent_index = curr_index / 2;

                nodes.insert((level + 1, parent_index), parent_hash.clone());

                curr_index = parent_index;
                curr_hash = parent_hash;
            }
        }

        let mut proof = Vec::new();
        let mut curr_index = index;

        for level in 0..self.tree_depth {
            let sibling_index = if curr_index % 2 == 0 {
                curr_index + 1
            } else {
                curr_index - 1
            };

            let sibling = nodes
                .get(&(level, sibling_index))
                .cloned()
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            proof.push(sibling);
            curr_index /= 2;
        }

        Ok(proof)
    }

    fn hash_pair(&self, a: &str, b: &str) -> Result<String> {
        use ethers::core::utils::keccak256;

        let a_bytes = hex::decode(a.trim_start_matches("0x"))?;
        let b_bytes = hex::decode(b.trim_start_matches("0x"))?;

        let hash = if a < b {
            keccak256([a_bytes, b_bytes].concat())
        } else {
            keccak256([b_bytes, a_bytes].concat())
        };

        Ok(format!("0x{}", hex::encode(hash)))
    }

    pub async fn get_mantle_root(&self) -> Result<String> {
        self.compute_mantle_commitment_root()
    }

    pub async fn get_ethereum_root(&self) -> Result<String> {
        self.compute_ethereum_root()
    }

    pub async fn get_tree_sizes(&self) -> Result<(usize, usize)> {
        let mantle = self.database.get_mantle_tree_size()?;
        let ethereum = self.database.get_ethereum_tree_size()?;
        Ok((mantle, ethereum))
    }
}

///    TESTS       ///
use serial_test::serial;
fn create_test_mantle_config() -> crate::relay_coordinator::model::MantleConfig {
    crate::relay_coordinator::model::MantleConfig {
        rpc_url: "https://rpc.sepolia.mantle.xyz".to_string(),
        ws_url: Some("ws://rpc.sepolia.mantle.xyz".to_string()),
        chain_id: 11155111,
        private_key: "0x2ea06215c638e5ac29dd5f2b894b936999e000888aace2400e691859e9d7fcba"
            .to_string(),
        intent_pool_address: "0x8e9080d32ae8864Af25D3fB59D28De74e7872b1d".to_string(),
        settlement_address: "0x985bD8f2348aB4b6d6279CA943ddcB932bAE0Bbd".to_string(),
    }
}

fn create_test_ethereum_config() -> crate::relay_coordinator::model::EthereumConfig {
    crate::relay_coordinator::model::EthereumConfig {
        rpc_url: "https://ethereum-sepolia-rpc.publicnode.com".to_string(),
        ws_url: Some("ws://ethereum-sepolia-rpc.publicnode.com".to_string()),
        chain_id: 11155111,
        private_key: "0x2ea06215c638e5ac29dd5f2b894b936999e000888aace2400e691859e9d7fcba"
            .to_string(),
        intent_pool_address: "0x759b40396ac6ff7f1d1cBe095507b5f65229b05a".to_string(),
        settlement_address: "0x86eEA33D59F1B5a806c41Cf7B040f507C8A6D7D7".to_string(),
    }
}

async fn setup_test_manager() -> MerkleTreeManager {
    let db = Arc::new(
        Database::new("postgresql://user:1234@localhost:5432/shadow-swap", 10)
            .expect("Failed to connect to test database"),
    );

    // Clean up any existing trees from previous test runs
    let _ = db.delete_merkle_tree_by_name("mantle_intents");
    let _ = db.delete_merkle_tree_by_name("ethereum_fills");
    let _ = db.delete_merkle_tree_by_name("ethereum_commitments");

    let mantle_config = create_test_mantle_config();
    let ethereum_config = create_test_ethereum_config();

    let mantle_relayer = Arc::new(
        MantleRelayer::new(mantle_config, db.clone())
            .await
            .expect("Failed to create Mantle relayer"),
    );

    let ethereum_relayer = Arc::new(
        EthereumRelayer::new(ethereum_config, db.clone())
            .await
            .expect("Failed to create Ethereum relayer"),
    );

    let manager = MerkleTreeManager::new(mantle_relayer, ethereum_relayer, db.clone(), 20);

    // Initialize the trees
    for (tree_name, depth) in &[
        ("mantle_intents", 20),
        ("ethereum_fills", 20),
        ("ethereum_commitments", 20),
    ] {
        db.ensure_merkle_tree(tree_name, *depth)
            .expect("Failed to ensure merkle tree");
    }

    manager
}

#[tokio::test]
#[serial]
async fn test_canonical_hashing() {
    let mgr = setup_test_manager().await;

    let a = "0x1111111111111111111111111111111111111111111111111111111111111111";
    let b = "0x2222222222222222222222222222222222222222222222222222222222222222";

    let h1 = mgr.hash_pair(a, b).unwrap();
    let h2 = mgr.hash_pair(b, a).unwrap();

    assert_eq!(h1, h2, "Canonical hashing must be order-independent");

    let h3 = mgr.hash_pair(a, b).unwrap();
    assert_eq!(h1, h3, "Hash should be deterministic");
}

#[tokio::test]
#[serial]
async fn test_incremental_matches_bulk() {
    let mgr = setup_test_manager().await;

    mgr.database.clear_mantle_tree().unwrap();
    mgr.database.clear_mantle_nodes().unwrap();

    let leaves: Vec<String> = vec![
        "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        "0x2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        "0x3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        "0x4444444444444444444444444444444444444444444444444444444444444444".to_string(),
    ];

    for leaf in &leaves {
        mgr.append_mantle_leaf(leaf).await.unwrap();
    }

    let incremental_root = mgr.compute_mantle_commitment_root().unwrap();

    // Compute from scratch
    let bulk_root = mgr.compute_root_from_leaves(&leaves).unwrap();

    assert_eq!(
        incremental_root, bulk_root,
        "Incremental tree building must produce same root as bulk computation"
    );
}

#[tokio::test]
#[serial]
async fn test_empty_tree() {
    let mgr = setup_test_manager().await;
    mgr.database.clear_mantle_tree().unwrap();

    let root = mgr.compute_mantle_commitment_root().unwrap();
    assert_eq!(root, ZERO_LEAF, "Empty tree should have zero root");
}

#[tokio::test]
#[serial]
async fn test_single_leaf_tree() {
    let mgr = setup_test_manager().await;
    mgr.database.clear_mantle_tree().unwrap();
    mgr.database.clear_mantle_nodes().unwrap();

    let leaf = "0x1111111111111111111111111111111111111111111111111111111111111111";
    mgr.append_mantle_leaf(leaf).await.unwrap();

    let root = mgr.compute_mantle_commitment_root().unwrap();
    assert_ne!(root, ZERO_LEAF);
    assert_ne!(root, leaf);
}

#[tokio::test]
#[serial]
async fn test_proof_generation_and_verification() {
    let mgr = setup_test_manager().await;
    mgr.database.clear_mantle_tree().unwrap();
    mgr.database.clear_mantle_nodes().unwrap();

    let leaves = vec![
        "0x1111111111111111111111111111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222222222222222222222222222",
        "0x3333333333333333333333333333333333333333333333333333333333333333",
    ];

    for leaf in &leaves {
        mgr.append_mantle_leaf(leaf).await.unwrap();
    }

    // Generate proof for second leaf
    let merkle_proof = mgr.generate_mantle_proof(leaves[1]).await.unwrap();

    assert_eq!(merkle_proof.leaf_index, 1);
    assert!(!merkle_proof.path.is_empty(), "Proof should not be empty");

    let expected_root = merkle_proof.root.clone();

    // Verify proof reconstructs to root
    let mut curr_hash = leaves[1].to_string();

    for sibling in &merkle_proof.path {
        curr_hash = mgr.hash_pair(&curr_hash, sibling).unwrap();
    }

    assert_eq!(curr_hash, expected_root, "Proof should reconstruct to root");
}

#[tokio::test]
#[serial]
async fn test_tree_sizes() {
    let mgr = setup_test_manager().await;
    mgr.database.clear_mantle_tree().unwrap();
    mgr.database.clear_ethereum_tree().unwrap();

    let (mantle_size, eth_size) = mgr.get_tree_sizes().await.unwrap();
    assert_eq!(mantle_size, 0);
    assert_eq!(eth_size, 0);

    mgr.append_mantle_leaf("0x1111111111111111111111111111111111111111111111111111111111111111")
        .await
        .unwrap();

    let (mantle_size, _) = mgr.get_tree_sizes().await.unwrap();
    assert_eq!(mantle_size, 1);
}

#[tokio::test]
#[serial]
async fn test_deterministic_roots() {
    let mgr = setup_test_manager().await;

    let leaves = vec![
        "0x1111111111111111111111111111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222222222222222222222222222",
    ];

    // Build tree once
    mgr.database.clear_mantle_tree().unwrap();
    mgr.database.clear_mantle_nodes().unwrap();

    for leaf in &leaves {
        mgr.append_mantle_leaf(leaf).await.unwrap();
    }
    let root1 = mgr.compute_mantle_commitment_root().unwrap();

    // Rebuild tree
    mgr.rebuild_mantle_tree().await.unwrap();
    let root2 = mgr.compute_mantle_commitment_root().unwrap();

    assert_eq!(root1, root2, "Rebuilding should produce same root");
}

#[tokio::test]
#[serial]
async fn test_hash_pair_with_zeros() {
    let mgr = setup_test_manager().await;

    let a = "0x1111111111111111111111111111111111111111111111111111111111111111";
    let zero = ZERO_LEAF;

    let h1 = mgr.hash_pair(a, zero).unwrap();
    let h2 = mgr.hash_pair(zero, a).unwrap();

    assert_eq!(h1, h2, "Hashing with zero should be order-independent");
}
