use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    database::database::Database,
    merkle_manager::proof_generator::MerkleProofGenerator,
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
    tree_locks: Arc<RwLock<()>>,
    proof_generator: Arc<MerkleProofGenerator>,
}

impl MerkleTreeManager {
    pub fn new(
        mantle_relayer: Arc<MantleRelayer>,
        ethereum_relayer: Arc<EthereumRelayer>,
        database: Arc<Database>,
        tree_depth: usize,
    ) -> Self {
        let proof_generator = Arc::new(MerkleProofGenerator::new(database.clone()));

        Self {
            mantle_relayer,
            ethereum_relayer,
            database,
            tree_depth,
            tree_locks: Arc::new(RwLock::new(())),
            proof_generator,
        }
    }

    /// Initialize all trees and rebuild from database
    pub async fn start(&self) -> Result<()> {
        info!("🌳 Merkle Tree Manager starting...");

        // Initialize all trees
        for (tree_name, depth) in &[
            ("mantle_intents", 20),
            ("mantle_commitments", 20),
            ("mantle_fills", 20),
            ("ethereum_intents", 20),
            ("ethereum_commitments", 20),
            ("ethereum_fills", 20),
        ] {
            self.database.ensure_merkle_tree(tree_name, *depth)?;
            info!("✅ Ensured tree '{}' exists", tree_name);
        }

        // Rebuild commitment trees from database
        info!("🔄 Rebuilding Mantle commitments tree...");
        self.rebuild_mantle_commitments_tree().await?;

        info!("🔄 Rebuilding Ethereum commitments tree...");
        self.rebuild_ethereum_commitments_tree().await?;

        info!("🔄 Rebuilding Mantle fills tree...");
        self.rebuild_mantle_fills_tree().await?;

        info!("🔄 Rebuilding Ethereum fills tree...");
        self.rebuild_ethereum_fills_tree().await?;

        // Verify consistency
        for tree_name in &["mantle_commitments", "ethereum_commitments"] {
            match self.database.verify_tree_consistency(tree_name) {
                Ok(true) => info!("✅ Tree '{}' is consistent", tree_name),
                Ok(false) => warn!("⚠️  Tree '{}' has inconsistent leaf count", tree_name),
                Err(e) => warn!("⚠️  Failed to verify tree '{}': {}", tree_name, e),
            }
        }

        info!("🌳 Merkle Tree Manager started successfully");

        // Keep running
        std::future::pending::<()>().await;
        Ok(())
    }

    /// Append a commitment to the appropriate tree based on chain ID
    pub async fn append_commitment(&self, commitment: &str, chain_id: u32) -> Result<usize> {
        let start = std::time::Instant::now();
        let _lock = self.tree_locks.write().await;

        let result = match chain_id {
            MANTLE_CHAIN_ID => {
                self.append_commitment_to_tree("mantle_commitments", commitment)
                    .await
            }
            ETHEREUM_CHAIN_ID => {
                self.append_commitment_to_tree("ethereum_commitments", commitment)
                    .await
            }
            _ => Err(anyhow!("Unsupported chain_id: {}", chain_id)),
        };

        info!("⏱️  append_commitment took {:?}", start.elapsed());
        result
    }

    pub async fn append_commitment_to_tree(
        &self,
        tree_name: &str,
        leaf_hash: &str,
    ) -> Result<usize> {
        let _lock = self.tree_locks.write().await;

        let tree = self
            .database
            .ensure_merkle_tree(tree_name, self.tree_depth as i32)?;
        let chain_name = if tree_name.contains("mantle") {
            "mantle"
        } else {
            "ethereum"
        };

        // ✅ FIX: Fetch ALL current leaves from database, not just up to leaf_count
        let mut leaves = self.database.get_all_commitments_for_chain(chain_name)?;

        // Check if leaf already exists
        if let Some(existing_index) = leaves
            .iter()
            .position(|l| l.to_lowercase() == leaf_hash.to_lowercase())
        {
            info!(
                "⚠️  Leaf {} already exists in tree '{}' at index {}",
                &leaf_hash[..10],
                tree_name,
                existing_index
            );
            return Ok(existing_index);
        }

        // Add new leaf
        let index = leaves.len();
        leaves.push(leaf_hash.to_string());

        // Compute new root with all leaves
        let new_root = self.compute_root_from_leaves(&leaves)?;

        // Update tree metadata atomically
        self.database.update_merkle_root(tree.tree_id, &new_root)?;

        // ✅ FIX: Set leaf count to ACTUAL count, not increment
        self.database
            .set_leaf_count(tree.tree_id, leaves.len() as i64)?;

        info!(
            "🌳 Tree '{}' updated: root={}, total_leaves={}",
            tree_name,
            &new_root[..10],
            leaves.len()
        );

        Ok(index)
    }

    pub async fn append_fill_to_tree(&self, tree_name: &str, intent_id: &str) -> Result<usize> {
        let _lock = self.tree_locks.write().await;

        let tree = self
            .database
            .ensure_merkle_tree(tree_name, self.tree_depth as i32)?;

        let chain_name = if tree_name.contains("mantle") {
            "mantle"
        } else {
            "ethereum"
        };

        let mut fills = self.database.get_all_fills_for_chain(chain_name)?;

        let index = if let Some(existing_index) = fills
            .iter()
            .position(|f| f.to_lowercase() == intent_id.to_lowercase())
        {
            info!(
                "⚠️  Fill {} already exists in tree '{}' at index {}, rebuilding tree anyway",
                &intent_id[..10],
                tree_name,
                existing_index
            );
            existing_index
        } else {
            let new_index = fills.len();
            fills.push(intent_id.to_string());
            new_index
        };

        info!(
            "🔄 Rebuilding fill tree '{}' with {} fills",
            tree_name,
            fills.len()
        );

        let new_root = self.compute_root_from_leaves(&fills)?;

        self.database.update_merkle_root(tree.tree_id, &new_root)?;
        self.database
            .set_leaf_count(tree.tree_id, fills.len() as i64)?;

        info!(
            "✅ Fill tree '{}' rebuilt: root={}, total_fills={}",
            tree_name,
            &new_root[..10],
            fills.len()
        );

        Ok(index)
    }

    /// Rebuild Mantle commitments tree from database
    pub async fn rebuild_mantle_commitments_tree(&self) -> Result<()> {
        let tree = self
            .database
            .ensure_merkle_tree("mantle_commitments", self.tree_depth as i32)?;

        self.rebuild_tree_from_chain(tree.tree_id, "mantle_commitments", "mantle")
            .await
    }

    /// Rebuild Ethereum commitments tree from database
    pub async fn rebuild_ethereum_commitments_tree(&self) -> Result<()> {
        let tree = self
            .database
            .ensure_merkle_tree("ethereum_commitments", self.tree_depth as i32)?;

        self.rebuild_tree_from_chain(tree.tree_id, "ethereum_commitments", "ethereum")
            .await
    }

    /// Rebuild Mantle intents tree
    pub async fn rebuild_mantle_intents_tree(&self) -> Result<()> {
        let tree = self
            .database
            .ensure_merkle_tree("mantle_intents", self.tree_depth as i32)?;

        self.rebuild_tree_from_chain(tree.tree_id, "mantle_intents", "mantle")
            .await
    }

    /// Rebuild Ethereum intents tree
    pub async fn rebuild_ethereum_intents_tree(&self) -> Result<()> {
        let tree = self
            .database
            .ensure_merkle_tree("ethereum_intents", self.tree_depth as i32)?;

        self.rebuild_tree_from_chain(tree.tree_id, "ethereum_intents", "ethereum")
            .await
    }

    pub async fn rebuild_mantle_fills_tree(&self) -> Result<()> {
        let tree = self
            .database
            .ensure_merkle_tree("mantle_fills", self.tree_depth as i32)?;

        let fills = self.database.get_all_fills_for_chain("mantle")?;

        self.rebuild_tree_from_leaves(tree.tree_id, "mantle_fills", fills)
            .await
    }

    pub async fn rebuild_ethereum_fills_tree(&self) -> Result<()> {
        let tree = self
            .database
            .ensure_merkle_tree("ethereum_fills", self.tree_depth as i32)?;

        let fills = self.database.get_all_fills_for_chain("ethereum")?;

        self.rebuild_tree_from_leaves(tree.tree_id, "ethereum_fills", fills)
            .await
    }

    /// Generic tree rebuild from chain commitments - FIXED VERSION
    async fn rebuild_tree_from_chain(
        &self,
        tree_id: i32,
        tree_name: &str,
        chain_name: &str,
    ) -> Result<()> {
        let _lock = self.tree_locks.write().await;

        info!(
            "🔄 Rebuilding tree '{}' from chain '{}'...",
            tree_name, chain_name
        );

        // ✅ FIX: Fetch ALL leaves from database, don't use limit
        let leaves = self.database.get_all_commitments_for_chain(chain_name)?;

        self.rebuild_tree_internal(tree_id, tree_name, leaves).await
    }

    /// Generic tree rebuild from provided leaves
    async fn rebuild_tree_from_leaves(
        &self,
        tree_id: i32,
        tree_name: &str,
        leaves: Vec<String>,
    ) -> Result<()> {
        let _lock = self.tree_locks.write().await;

        info!(
            "🔄 Rebuilding tree '{}' from {} leaves...",
            tree_name,
            leaves.len()
        );

        self.rebuild_tree_internal(tree_id, tree_name, leaves).await
    }

    async fn rebuild_tree_internal(
        &self,
        tree_id: i32,
        tree_name: &str,
        leaves: Vec<String>,
    ) -> Result<()> {
        self.database.clear_merkle_nodes_by_tree(tree_id)?;

        if leaves.is_empty() {
            info!(
                "⚠️  Tree '{}' has no leaves, setting to zero root",
                tree_name
            );
            self.database.update_merkle_root(tree_id, ZERO_LEAF)?;
            self.database.set_leaf_count(tree_id, 0)?;
            return Ok(());
        }

        info!(
            "📊 Building tree '{}' with {} leaves",
            tree_name,
            leaves.len()
        );

        let tree_size = std::cmp::max(2, Self::next_power_of_2(leaves.len()));
        let mut current_layer = leaves.clone();
        current_layer.resize(tree_size, ZERO_LEAF.to_string());

        let mut level = 0;
        let mut current_size = tree_size;

        while current_size > 0 {
            for (idx, hash) in current_layer.iter().enumerate() {
                self.database
                    .store_merkle_node(tree_id, level, idx as i64, hash)?;
            }

            if current_size == 1 {
                break;
            }

            let mut next_layer = Vec::with_capacity(current_size / 2);
            for i in 0..(current_size / 2) {
                next_layer.push(self.hash_pair(&current_layer[2 * i], &current_layer[2 * i + 1])?);
            }

            current_layer = next_layer;
            current_size /= 2;
            level += 1;
        }

        let root = &current_layer[0];

        self.database.update_merkle_root(tree_id, root)?;
        self.database.set_leaf_count(tree_id, leaves.len() as i64)?;

        info!(
            "✅ Tree '{}' rebuilt: root={}, leaves={}",
            tree_name,
            &root[..10],
            leaves.len()
        );

        Ok(())
    }

    fn compute_root_from_leaves(&self, leaves: &[String]) -> Result<String> {
        if leaves.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        let tree_size = std::cmp::max(2, Self::next_power_of_2(leaves.len()));
        let mut layer: Vec<String> = leaves.to_vec();
        layer.resize(tree_size, ZERO_LEAF.to_string());

        while layer.len() > 1 {
            let mut next_layer = Vec::with_capacity(layer.len() / 2);
            for i in 0..(layer.len() / 2) {
                next_layer.push(self.hash_pair(&layer[2 * i], &layer[2 * i + 1])?);
            }
            layer = next_layer;
        }

        Ok(layer[0].clone())
    }

    /// Get commitment proof with specific tree size
    pub async fn get_commitment_proof(
        &self,
        commitment: &str,
        chain_name: &str,
        limit: usize,
    ) -> Result<(Vec<String>, u32)> {
        let (proof, index, _root) = self
            .proof_generator
            .generate_proof(chain_name, commitment, limit)?;
        Ok((proof, index as u32))
    }

    // Root getters
    pub async fn get_mantle_intents_root(&self) -> Result<String> {
        self.database
            .get_latest_root("mantle_intents")?
            .ok_or_else(|| anyhow!("No Mantle intents root found"))
    }

    pub async fn get_mantle_fills_root(&self) -> Result<String> {
        self.database
            .get_latest_root("mantle_fills")?
            .ok_or_else(|| anyhow!("No Mantle fills root found"))
    }

    pub async fn get_mantle_commitments_root(&self) -> Result<String> {
        self.database
            .get_latest_root("mantle_commitments")?
            .ok_or_else(|| anyhow!("No Mantle commitments root found"))
    }

    pub async fn get_ethereum_fills_root(&self) -> Result<String> {
        self.database
            .get_latest_root("ethereum_fills")?
            .ok_or_else(|| anyhow!("No Ethereum fills root found"))
    }

    pub async fn get_ethereum_commitments_root(&self) -> Result<String> {
        self.database
            .get_latest_root("ethereum_commitments")?
            .ok_or_else(|| anyhow!("No Ethereum commitments root found"))
    }

    pub fn get_all_mantle_fills(&self) -> Result<Vec<String>> {
        self.database.get_all_fills_for_chain("mantle")
    }

    /// Get all Ethereum fills from database (for proof generation)
    pub fn get_all_ethereum_fills(&self) -> Result<Vec<String>> {
        self.database.get_all_fills_for_chain("ethereum")
    }

    pub fn compute_ethereum_fills_root(&self) -> Result<String> {
        let tree = self
            .database
            .get_merkle_tree_by_name("ethereum_fills")?
            .ok_or_else(|| anyhow!("Ethereum fills tree not found"))?;
        Ok(tree.root)
    }

    pub fn compute_mantle_fills_root(&self) -> Result<String> {
        let tree = self
            .database
            .get_merkle_tree_by_name("mantle_fills")?
            .ok_or_else(|| anyhow!("Mantle fills tree not found"))?;
        Ok(tree.root)
    }

    pub fn compute_mantle_intents_root(&self) -> Result<String> {
        let tree = self
            .database
            .get_merkle_tree_by_name("mantle_intents")?
            .ok_or_else(|| anyhow!("Mantle intents tree not found"))?;
        Ok(tree.root)
    }

    pub fn compute_mantle_commitments_root(&self) -> Result<String> {
        self.proof_generator.compute_mantle_root()
    }

    pub fn compute_ethereum_intents_root(&self) -> Result<String> {
        let tree = self
            .database
            .get_merkle_tree_by_name("ethereum_intents")?
            .ok_or_else(|| anyhow!("Ethereum intents tree not found"))?;
        Ok(tree.root)
    }

    pub fn compute_ethereum_commitments_root(&self) -> Result<String> {
        self.proof_generator.compute_ethereum_root()
    }

    pub async fn get_mantle_fill_proof(
        &self,
        intent_id: &str,
        limit: usize,
    ) -> Result<(Vec<String>, u32)> {
        // Get fills instead of commitments for fill tree
        let fills = self.database.get_fills_for_tree("mantle", limit as i64)?;

        // Find intent_id position in fills
        let index = fills
            .iter()
            .position(|f| f.to_lowercase() == intent_id.to_lowercase())
            .ok_or_else(|| anyhow!("Intent {} not found in mantle fills", intent_id))?;

        // Generate proof using the same proof generator logic
        let (proof, index, _root) = self
            .proof_generator
            .generate_fill_proof("mantle", intent_id, limit)?;

        Ok((proof, index as u32))
    }

    pub async fn get_ethereum_fill_proof(
        &self,
        intent_id: &str,
        limit: usize,
    ) -> Result<(Vec<String>, u32)> {
        let fills = self.database.get_fills_for_tree("ethereum", limit as i64)?;

        let index = fills
            .iter()
            .position(|f| f.to_lowercase() == intent_id.to_lowercase())
            .ok_or_else(|| anyhow!("Intent {} not found in ethereum fills", intent_id))?;

        let (proof, index, _root) = self
            .proof_generator
            .generate_fill_proof("ethereum", intent_id, limit)?;

        Ok((proof, index as u32))
    }

    pub async fn get_tree_sizes(&self) -> Result<(usize, usize, usize, usize)> {
        let mantle_intents = self.database.get_tree_size("mantle_intents")?;
        let mantle_commitments = self.database.get_tree_size("mantle_commitments")?;
        let ethereum_fills = self.database.get_tree_size("ethereum_fills")?;
        let ethereum_commitments = self.database.get_tree_size("ethereum_commitments")?;

        Ok((
            mantle_intents,
            mantle_commitments,
            ethereum_fills,
            ethereum_commitments,
        ))
    }

    pub fn get_proof_generator(&self) -> Arc<MerkleProofGenerator> {
        self.proof_generator.clone()
    }

    /// Hash a pair of nodes (sorted)
    fn hash_pair(&self, a: &str, b: &str) -> Result<String> {
        use ethers::core::utils::keccak256;
        use ethers::types::H256;

        let a_bytes = H256::from_slice(&hex::decode(a.trim_start_matches("0x"))?);
        let b_bytes = H256::from_slice(&hex::decode(b.trim_start_matches("0x"))?);

        let hash = if a_bytes < b_bytes {
            let mut concat = [0u8; 64];
            concat[..32].copy_from_slice(a_bytes.as_bytes());
            concat[32..].copy_from_slice(b_bytes.as_bytes());
            keccak256(concat)
        } else {
            let mut concat = [0u8; 64];
            concat[..32].copy_from_slice(b_bytes.as_bytes());
            concat[32..].copy_from_slice(a_bytes.as_bytes());
            keccak256(concat)
        };

        Ok(format!("0x{}", hex::encode(hash)))
    }

    /// Calculate next power of 2
    fn next_power_of_2(n: usize) -> usize {
        if n == 0 {
            return 1;
        }
        let mut p = n - 1;
        p |= p >> 1;
        p |= p >> 2;
        p |= p >> 4;
        p |= p >> 8;
        p |= p >> 16;
        p |= p >> 32;
        p + 1
    }
}
