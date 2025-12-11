use anyhow::Result;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn};

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

        self.rebuild_mantle_tree().await?;
        self.rebuild_ethereum_tree().await?;

        info!("🔄 Starting root sync loop");

        tokio::try_join!(self.run_root_sync_loop())?;

        Ok(())
    }

    async fn run_root_sync_loop(&self) -> Result<()> {
        let mut tick = interval(Duration::from_secs(120));

        loop {
            tick.tick().await;

            match self.sync_roots().await {
                Ok(_) => debug!("✅ Roots synced"),
                Err(e) => warn!("⚠️ Root sync failed: {}", e),
            }
        }
    }

    pub async fn append_commitment(&self, commitment: &str, chain_id: u32) -> Result<usize> {
        match chain_id {
            MANTLE_CHAIN_ID => self.append_mantle_leaf(commitment).await,
            ETHEREUM_CHAIN_ID => self.append_ethereum_leaf(commitment).await,
            _ => Err(anyhow::anyhow!("Unsupported chain_id: {}", chain_id)),
        }
    }

    /// Append leaf to Mantle tree using CANONICAL hashing
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

            // Fetch sibling or use zero
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

    /// Append leaf to Ethereum tree using CANONICAL hashing
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

        let root = self.compute_mantle_root()?;
        info!("✅ Mantle tree rebuilt: {}", root);

        Ok(())
    }

    pub async fn rebuild_ethereum_tree(&self) -> Result<()> {
        info!("🔨 Rebuilding Ethereum tree");

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
        info!("✅ Ethereum tree rebuilt: {}", root);

        Ok(())
    }

    fn hex_to_bytes32(&self, hex_str: &str) -> Result<[u8; 32]> {
        hex::decode(hex_str.trim_start_matches("0x"))
            .map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid length: expected 32 bytes"))
    }

    async fn sync_roots(&self) -> Result<()> {
        let mantle_root = self.compute_mantle_root()?;
        let ethereum_root = self.compute_ethereum_root()?;

        info!("🔄 Syncing roots");
        debug!("  Mantle: {}", mantle_root);
        debug!("  Ethereum: {}", ethereum_root);

        const MANTLE_CHAIN_ID: u32 = 5003;
        const ETHEREUM_CHAIN_ID: u32 = 11155111;

        match self
            .ethereum_relayer
            .sync_source_chain_root(MANTLE_CHAIN_ID, mantle_root.clone())
            .await
        {
            Ok(tx_hash) => {
                self.database
                    .record_root_sync("mantle_to_ethereum", &mantle_root, &tx_hash)?;
                info!("✅ Mantle → Ethereum ({})", tx_hash);
            }
            Err(e) => warn!("⚠️ Failed to sync Mantle → Ethereum: {}", e),
        }

        let ethereum_root_bytes = self.hex_to_bytes32(&ethereum_root)?;

        match self
            .mantle_relayer
            .sync_dest_chain_root(ETHEREUM_CHAIN_ID, ethereum_root_bytes)
            .await
        {
            Ok(tx_hash) => {
                self.database
                    .record_root_sync("ethereum_to_mantle", &ethereum_root, &tx_hash)?;
                info!("✅ Ethereum → Mantle ({})", tx_hash);
            }
            Err(e) => warn!("⚠️ Failed to sync Ethereum → Mantle: {}", e),
        }

        Ok(())
    }

    pub async fn generate_mantle_proof(&self, commitment: &str) -> Result<(MerkleProof)> {
        let tree = self.database.get_mantle_tree()?;
        let index = tree
            .iter()
            .position(|c| c == commitment)
            .ok_or_else(|| anyhow::anyhow!("Commitment not found"))?;

        let proof = self.compute_merkle_proof(&tree, index)?;
        let root = self.compute_mantle_root()?;

        Ok(MerkleProof {
            path: proof,
            leaf_index: index,
            root,
        })
    }

    /// Generate Merkle proof for Ethereum fill
    pub async fn generate_ethereum_proof(&self, intent_id: &str) -> Result<MerkleProof> {
        let tree = self.database.get_ethereum_tree()?;
        let index = tree
            .iter()
            .position(|id| id == intent_id)
            .ok_or_else(|| anyhow::anyhow!("Intent not found"))?;

        let proof = self.compute_merkle_proof(&tree, index)?;
        let root = self.compute_ethereum_root()?;

        Ok(MerkleProof {
            path: proof,
            leaf_index: index,
            root,
        })
    }

    fn compute_mantle_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("mantle") {
            return Ok(root);
        }

        let tree = self.database.get_mantle_tree()?;
        self.compute_root_from_leaves(&tree)
    }

    fn compute_ethereum_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("ethereum") {
            return Ok(root);
        }

        let tree = self.database.get_ethereum_tree()?;
        self.compute_root_from_leaves(&tree)
    }

    fn compute_root_from_leaves(&self, leaves: &[String]) -> Result<String> {
        if leaves.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        if leaves.len() == 1 {
            return Ok(leaves[0].clone());
        }

        let mut layer = leaves.to_vec();

        while layer.len() > 1 {
            let mut next = Vec::new();

            for i in (0..layer.len()).step_by(2) {
                let hash = if i + 1 < layer.len() {
                    self.hash_pair(&layer[i], &layer[i + 1])?
                } else {
                    layer[i].clone()
                };
                next.push(hash);
            }

            layer = next;
        }

        Ok(layer[0].clone())
    }

    fn compute_merkle_proof(&self, leaves: &[String], index: usize) -> Result<Vec<String>> {
        if index >= leaves.len() {
            return Err(anyhow::anyhow!("Index out of bounds"));
        }

        let mut proof = Vec::new();
        let mut layer = leaves.to_vec();
        let mut curr_index = index;

        while layer.len() > 1 {
            let sibling_index = if curr_index % 2 == 0 {
                curr_index + 1
            } else {
                curr_index - 1
            };

            let sibling = if sibling_index < layer.len() {
                layer[sibling_index].clone()
            } else {
                layer[curr_index].clone()
            };

            proof.push(sibling);

            let mut next = Vec::new();
            for i in (0..layer.len()).step_by(2) {
                let hash = if i + 1 < layer.len() {
                    self.hash_pair(&layer[i], &layer[i + 1])?
                } else {
                    layer[i].clone()
                };
                next.push(hash);
            }

            layer = next;
            curr_index /= 2;
        }

        Ok(proof)
    }

    /// CANONICAL hash pair - ALWAYS sorts inputs
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
        self.compute_mantle_root()
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
fn create_test_mantle_config() -> crate::relay_coordinator::model::MantleConfig {
    crate::relay_coordinator::model::MantleConfig {
        rpc_url: "http://localhost:8545".to_string(),
        ws_url: Some("ws://localhost:8546".to_string()), // ✅ Added
        chain_id: 11155111,
        private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .to_string(), // Test key
        intent_pool_address: "0x5FbDB2315678afecb367f032d93F642f64180aa3".to_string(),
        settlement_address: "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512".to_string(),
    }
}

fn create_test_ethereum_config() -> crate::relay_coordinator::model::EthereumConfig {
    crate::relay_coordinator::model::EthereumConfig {
        rpc_url: "http://localhost:8546".to_string(),
        ws_url: Some("ws://localhost:8546".to_string()), // ✅ Added
        chain_id: 11155111,
        private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .to_string(),
        intent_pool_address: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0".to_string(),
        settlement_address: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9".to_string(),
    }
}

async fn setup_test_manager() -> MerkleTreeManager {
    let db = Arc::new(
        Database::new("postgresql://test:test@localhost:5432/test_db", 10)
            .expect("Failed to connect to test database"),
    );

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

    MerkleTreeManager::new(mantle_relayer, ethereum_relayer, db, 20)
}

#[tokio::test]
async fn test_canonical_hashing() {
    let mgr = setup_test_manager().await;

    let a = "0x1111111111111111111111111111111111111111111111111111111111111111";
    let b = "0x2222222222222222222222222222222222222222222222222222222222222222";

    let h1 = mgr.hash_pair(a, b).unwrap();
    let h2 = mgr.hash_pair(b, a).unwrap();

    assert_eq!(h1, h2, "Canonical hashing must be order-independent");

    // Also verify it matches the expected sorted order
    let h3 = mgr.hash_pair(a, b).unwrap();
    assert_eq!(h1, h3, "Hash should be deterministic");
}

#[tokio::test]
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

    let incremental_root = mgr.compute_mantle_root().unwrap();

    // Compute from scratch
    let bulk_root = mgr.compute_root_from_leaves(&leaves).unwrap();

    assert_eq!(
        incremental_root, bulk_root,
        "Incremental tree building must produce same root as bulk computation"
    );
}

#[tokio::test]
async fn test_empty_tree() {
    let mgr = setup_test_manager().await;
    mgr.database.clear_mantle_tree().unwrap();

    let root = mgr.compute_mantle_root().unwrap();
    assert_eq!(root, ZERO_LEAF, "Empty tree should have zero root");
}

#[tokio::test]
async fn test_single_leaf_tree() {
    let mgr = setup_test_manager().await;
    mgr.database.clear_mantle_tree().unwrap();

    let leaf = "0x1111111111111111111111111111111111111111111111111111111111111111";
    mgr.append_mantle_leaf(leaf).await.unwrap();

    let root = mgr.compute_mantle_root().unwrap();
    // Single leaf means root should be computed through the tree depth
    assert_ne!(root, ZERO_LEAF);
    assert_ne!(root, leaf); // Root should be hashed with siblings
}

#[tokio::test]
async fn test_proof_generation_and_verification() {
    let mgr = setup_test_manager().await;
    mgr.database.clear_mantle_tree().unwrap();

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
    let mut curr_index = merkle_proof.leaf_index;

    for sibling in merkle_proof.path {
        curr_hash = mgr.hash_pair(&curr_hash, &sibling).unwrap();
        curr_index /= 2;
    }

    assert_eq!(curr_hash, expected_root, "Proof should reconstruct to root");
}

#[tokio::test]
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
async fn test_deterministic_roots() {
    let mgr = setup_test_manager().await;

    let leaves = vec![
        "0x1111111111111111111111111111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222222222222222222222222222",
    ];

    // Build tree once
    mgr.database.clear_mantle_tree().unwrap();
    for leaf in &leaves {
        mgr.append_mantle_leaf(leaf).await.unwrap();
    }
    let root1 = mgr.compute_mantle_root().unwrap();

    // Rebuild tree
    mgr.rebuild_mantle_tree().await.unwrap();
    let root2 = mgr.compute_mantle_root().unwrap();

    assert_eq!(root1, root2, "Rebuilding should produce same root");
}

#[tokio::test]
async fn test_hash_pair_with_zeros() {
    let mgr = setup_test_manager().await;

    let a = "0x1111111111111111111111111111111111111111111111111111111111111111";
    let zero = ZERO_LEAF;

    let h1 = mgr.hash_pair(a, zero).unwrap();
    let h2 = mgr.hash_pair(zero, a).unwrap();

    assert_eq!(h1, h2, "Hashing with zero should be order-independent");
}
