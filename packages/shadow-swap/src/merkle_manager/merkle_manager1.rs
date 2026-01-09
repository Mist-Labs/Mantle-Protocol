use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::info;

use crate::{
    database::database::Database,
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
            ("mantle_commitments", 20),
            ("mantle_fills", 20),
            ("ethereum_intents", 20),
            ("ethereum_commitments", 20),
            ("ethereum_fills", 20),
        ] {
            self.database.ensure_merkle_tree(tree_name, *depth)?;
        }

        loop {
            let _ = self.rebuild_mantle_intents_tree().await;
            let _ = self.rebuild_mantle_commitments_tree().await;
            let _ = self.rebuild_mantle_fills_tree().await;
            let _ = self.rebuild_ethereum_intents_tree().await;
            let _ = self.rebuild_ethereum_commitments_tree().await;
            let _ = self.rebuild_ethereum_fills_tree().await;

            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    pub async fn append_commitment(&self, commitment: &str, chain_id: u32) -> Result<usize> {
        match chain_id {
            MANTLE_CHAIN_ID => self.append_mantle_commitment_leaf(commitment).await,
            ETHEREUM_CHAIN_ID => self.append_ethereum_commitment_leaf(commitment).await,
            _ => Err(anyhow::anyhow!("Unsupported chain_id: {}", chain_id)),
        }
    }

    pub async fn append_leaf_to_tree(&self, tree_name: &str, leaf_hash: &str) -> Result<usize> {
        let tree = self
            .database
            .ensure_merkle_tree(tree_name, self.tree_depth as i32)?;
        let index = tree.leaf_count as usize;

        self.database
            .store_merkle_node(tree.tree_id, 0, index as i64, leaf_hash)?;

        let mut curr_index = index;
        let mut curr_hash = leaf_hash.to_string();

        for level in 0..self.tree_depth {
            let sibling_index = if curr_index % 2 == 0 {
                curr_index + 1
            } else {
                curr_index - 1
            };

            let sibling = self
                .database
                .get_merkle_node(tree.tree_id, level as i32, sibling_index as i64)?
                .map(|n| n.hash)
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;
            let parent_index = curr_index / 2;

            self.database.store_merkle_node(
                tree.tree_id,
                (level + 1) as i32,
                parent_index as i64,
                &parent_hash,
            )?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database.update_merkle_root(tree.tree_id, &curr_hash)?;
        self.database.increment_leaf_count(tree.tree_id, 1)?;

        Ok(index)
    }

    pub async fn append_mantle_leaf(&self, commitment: &str) -> Result<usize> {
        let tree = self
            .database
            .ensure_merkle_tree("mantle_intents", self.tree_depth as i32)?;
        let index = tree.leaf_count as usize;

        self.database
            .store_merkle_node(tree.tree_id, 0, index as i64, commitment)?;

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
                .get_merkle_node(tree.tree_id, level as i32, sibling_index as i64)?
                .map(|n| n.hash)
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;
            let parent_index = curr_index / 2;

            self.database.store_merkle_node(
                tree.tree_id,
                (level + 1) as i32,
                parent_index as i64,
                &parent_hash,
            )?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database.update_merkle_root(tree.tree_id, &curr_hash)?;
        self.database.increment_leaf_count(tree.tree_id, 1)?;

        Ok(index)
    }

    pub async fn rebuild_mantle_intents_tree(&self) -> Result<()> {
        let commitments = self.database.get_all_commitments_for_chain("mantle")?;

        let tree = self
            .database
            .ensure_merkle_tree("mantle_intents", self.tree_depth as i32)?;

        if commitments.len() == tree.leaf_count as usize && !commitments.is_empty() {
            return Ok(());
        }

        info!(
            "🔨 Syncing Mantle intents tree ({} leaves)",
            commitments.len()
        );

        if commitments.is_empty() {
            self.database.update_merkle_root(tree.tree_id, ZERO_LEAF)?;
            self.database.reset_leaf_count(tree.tree_id)?;
            return Ok(());
        }

        self.database.clear_merkle_nodes_by_tree(tree.tree_id)?;
        self.database.reset_leaf_count(tree.tree_id)?;

        for commitment in commitments {
            self.append_leaf_to_tree("mantle_intents", &commitment)
                .await?;
        }

        info!("✅ Mantle intents tree sync complete.");
        Ok(())
    }

    pub fn compute_mantle_intents_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("mantle_intents") {
            return Ok(root);
        }

        let tree = self.database.get_mantle_tree()?;

        if tree.is_empty() {
            self.database.record_root("mantle_intents", ZERO_LEAF)?;
            return Ok(ZERO_LEAF.to_string());
        }

        let root = self.compute_root_from_leaves(&tree)?;
        self.database.record_root("mantle_intents", &root)?;

        Ok(root)
    }

    pub async fn append_mantle_commitment_leaf(&self, commitment: &str) -> Result<usize> {
        let size = self.database.get_mantle_commitment_tree_size()?;
        let index = size;

        self.database.add_to_mantle_commitment_tree(commitment)?;
        self.database
            .set_mantle_commitment_node(0, index, commitment)?;

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
                .get_mantle_commitment_node(level, sibling_index)?
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;

            let parent_index = curr_index / 2;
            self.database
                .set_mantle_commitment_node(level + 1, parent_index, &parent_hash)?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database
            .record_root("mantle_commitments", &curr_hash)?;
        info!("✅ Mantle commitments root: {}", curr_hash);

        Ok(index)
    }

    pub async fn rebuild_mantle_commitments_tree(&self) -> Result<()> {
        let commitments = self.database.get_all_commitments_for_chain("mantle")?;
        let current_size = self.database.get_mantle_commitment_tree_size()?;

        if commitments.len() == current_size && !commitments.is_empty() {
            return Ok(());
        }

        info!("🔨 Rebuilding Mantle commitments tree");

        if commitments.is_empty() {
            self.database.record_root("mantle_commitments", ZERO_LEAF)?;
            return Ok(());
        }

        self.database.clear_mantle_commitment_tree()?;
        self.database.clear_mantle_commitment_nodes()?;

        for commitment in commitments {
            self.append_mantle_commitment_leaf(&commitment).await?;
        }
        Ok(())
    }

    pub fn compute_mantle_commitments_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("mantle_commitments") {
            return Ok(root);
        }

        let tree = self.database.get_all_mantle_commitments()?;

        if tree.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        let root = self.compute_root_from_leaves(&tree)?;
        self.database.record_root("mantle_commitments", &root)?;

        Ok(root)
    }

    pub async fn rebuild_mantle_fills_tree(&self) -> Result<()> {
        let sorted_fills = self.database.get_all_mantle_fills()?;
        let tree = self
            .database
            .ensure_merkle_tree("mantle_fills", self.tree_depth as i32)?;

        if sorted_fills.len() == tree.leaf_count as usize && !sorted_fills.is_empty() {
            return Ok(());
        }

        info!(
            "🔨 Syncing Mantle fills tree ({} leaves)",
            sorted_fills.len()
        );

        if sorted_fills.is_empty() {
            self.database.update_merkle_root(tree.tree_id, ZERO_LEAF)?;
            self.database.reset_leaf_count(tree.tree_id)?;
            return Ok(());
        }

        self.database.clear_merkle_nodes_by_tree(tree.tree_id)?;
        self.database.reset_leaf_count(tree.tree_id)?;

        for fill in sorted_fills {
            self.append_leaf_to_tree("mantle_fills", &fill.intent_id)
                .await?;
        }

        Ok(())
    }

    pub async fn append_mantle_fill_leaf(&self, intent_id: &str) -> Result<usize> {
        let tree = self
            .database
            .ensure_merkle_tree("mantle_fills", self.tree_depth as i32)?;
        let index = tree.leaf_count as usize;

        self.database
            .store_merkle_node(tree.tree_id, 0, index as i64, intent_id)?;

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
                .get_merkle_node(tree.tree_id, level as i32, sibling_index as i64)?
                .map(|n| n.hash)
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;
            let parent_index = curr_index / 2;

            self.database.store_merkle_node(
                tree.tree_id,
                (level + 1) as i32,
                parent_index as i64,
                &parent_hash,
            )?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database.update_merkle_root(tree.tree_id, &curr_hash)?;
        self.database.increment_leaf_count(tree.tree_id, 1)?;

        Ok(index)
    }

    pub async fn append_ethereum_intent_leaf(&self, commitment: &str) -> Result<usize> {
        let size = self.database.get_ethereum_intent_tree_size()?;
        let index = size;

        self.database.add_to_ethereum_intent_tree(commitment)?;
        self.database
            .set_ethereum_intent_node(0, index, commitment)?;

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
                .get_ethereum_intent_node(level, sibling_index)?
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;

            let parent_index = curr_index / 2;
            self.database
                .set_ethereum_intent_node(level + 1, parent_index, &parent_hash)?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database.record_root("ethereum_intents", &curr_hash)?;
        info!("✅ Ethereum intents root: {}", curr_hash);

        Ok(index)
    }

    pub async fn rebuild_ethereum_intents_tree(&self) -> Result<()> {
        let commitments = self.database.get_all_commitments_for_chain("ethereum")?;
        let current_size = self.database.get_ethereum_intent_tree_size()?;

        if commitments.len() == current_size && !commitments.is_empty() {
            return Ok(());
        }

        info!("🔨 Rebuilding Ethereum intents tree");

        if commitments.is_empty() {
            self.database.record_root("ethereum_intents", ZERO_LEAF)?;
            return Ok(());
        }

        self.database.clear_ethereum_intent_tree()?;
        self.database.clear_ethereum_intent_nodes()?;

        for commitment in commitments {
            self.append_ethereum_intent_leaf(&commitment).await?;
        }

        let root = self.compute_ethereum_intents_root()?;
        info!("✅ Ethereum intents tree rebuilt: {}", root);

        Ok(())
    }

    pub fn compute_ethereum_intents_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("ethereum_intents") {
            return Ok(root);
        }

        let tree = self.database.get_ethereum_intent_tree()?;

        if tree.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        let root = self.compute_root_from_leaves(&tree)?;
        self.database.record_root("ethereum_intents", &root)?;

        Ok(root)
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
        info!("✅ Ethereum commitments root: {}", curr_hash);

        Ok(index)
    }

    pub async fn rebuild_ethereum_commitments_tree(&self) -> Result<()> {
        let commitments = self.database.get_all_commitments_for_chain("ethereum")?;
        let current_size = self.database.get_ethereum_commitment_tree_size()?;

        if commitments.len() == current_size && !commitments.is_empty() {
            return Ok(());
        }

        info!("🔨 Rebuilding Ethereum commitments tree");

        if commitments.is_empty() {
            self.database
                .record_root("ethereum_commitments", ZERO_LEAF)?;
            return Ok(());
        }

        self.database.clear_ethereum_commitment_tree()?;
        self.database.clear_ethereum_commitment_nodes()?;

        for commitment in commitments {
            self.append_ethereum_commitment_leaf(&commitment).await?;
        }
        Ok(())
    }

    pub fn compute_ethereum_commitments_root(&self) -> Result<String> {
        if let Ok(Some(root)) = self.database.get_latest_root("ethereum_commitments") {
            return Ok(root);
        }

        let tree = self.database.get_all_ethereum_commitments()?;

        if tree.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        let root = self.compute_root_from_leaves(&tree)?;
        self.database.record_root("ethereum_commitments", &root)?;

        Ok(root)
    }

    pub async fn append_ethereum_fill_leaf(&self, intent_id: &str) -> Result<usize> {
        let tree = self
            .database
            .ensure_merkle_tree("ethereum_fills", self.tree_depth as i32)?;
        let index = tree.leaf_count as usize;

        self.database
            .store_merkle_node(tree.tree_id, 0, index as i64, intent_id)?;

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
                .get_merkle_node(tree.tree_id, level as i32, sibling_index as i64)?
                .map(|n| n.hash)
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let parent_hash = self.hash_pair(&curr_hash, &sibling)?;
            let parent_index = curr_index / 2;

            self.database.store_merkle_node(
                tree.tree_id,
                (level + 1) as i32,
                parent_index as i64,
                &parent_hash,
            )?;

            curr_index = parent_index;
            curr_hash = parent_hash;
        }

        self.database.update_merkle_root(tree.tree_id, &curr_hash)?;
        self.database.increment_leaf_count(tree.tree_id, 1)?;

        Ok(index)
    }

    pub async fn rebuild_ethereum_fills_tree(&self) -> Result<()> {
        let sorted_fills = self.database.get_all_ethereum_fills()?;
        let tree = self
            .database
            .ensure_merkle_tree("ethereum_fills", self.tree_depth as i32)?;

        if sorted_fills.len() == tree.leaf_count as usize && !sorted_fills.is_empty() {
            return Ok(());
        }

        info!(
            "🔨 Syncing Ethereum fills tree ({} leaves)",
            sorted_fills.len()
        );

        if sorted_fills.is_empty() {
            self.database.update_merkle_root(tree.tree_id, ZERO_LEAF)?;
            self.database.reset_leaf_count(tree.tree_id)?;
            return Ok(());
        }

        self.database.clear_merkle_nodes_by_tree(tree.tree_id)?;
        self.database.reset_leaf_count(tree.tree_id)?;

        for fill in sorted_fills {
            self.append_leaf_to_tree("ethereum_fills", &fill.intent_id)
                .await?;
        }

        Ok(())
    }

    // pub fn compute_ethereum_fills_root(&self) -> Result<String> {
    //     if let Ok(Some(root)) = self.database.get_latest_root("ethereum_fills") {
    //         return Ok(root);
    //     }

    //     let tree = self.database.get_ethereum_fill_tree()?;

    //     if tree.is_empty() {
    //         return Ok(ZERO_LEAF.to_string());
    //     }

    //     let root = self.compute_root_from_leaves(&tree)?;
    //     self.database.record_root("ethereum_fills", &root)?;

    //     Ok(root)
    // }

    pub fn compute_ethereum_fills_root(&self) -> Result<String> {
        let tree = self
            .database
            .get_merkle_tree_by_name("ethereum_fills")?
            .ok_or_else(|| anyhow!("Ethereum fills tree not found"))?;
        Ok(tree.root)
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

    fn hash_pair(&self, a: &str, b: &str) -> Result<String> {
        use ethers::core::utils::keccak256;

        let a_bytes = hex::decode(a.trim_start_matches("0x"))
            .map_err(|_| anyhow!("Invalid hex leaf A: {}", a))?;
        let b_bytes = hex::decode(b.trim_start_matches("0x"))
            .map_err(|_| anyhow!("Invalid hex leaf B: {}", b))?;

        let hash = if a_bytes < b_bytes {
            keccak256([a_bytes, b_bytes].concat())
        } else {
            keccak256([b_bytes, a_bytes].concat())
        };

        Ok(format!("0x{}", hex::encode(hash)))
    }

    pub async fn get_commitment_proof(
        &self,
        commitment: &str,
        chain_name: &str,
    ) -> Result<(Vec<String>, u32)> {
        let leaves = self.database.get_all_commitments_for_chain(chain_name)?;

        if leaves.is_empty() {
            return Err(anyhow!("Merkle tree for {} is empty", chain_name));
        }

        let root_key = match chain_name.to_lowercase().as_str() {
            "ethereum" => "ethereum_commitments",
            "mantle" => "mantle_commitments",
            _ => return Err(anyhow!("Unsupported chain: {}", chain_name)),
        };

        let synced_root = self
            .database
            .get_latest_root(root_key)?
            .ok_or_else(|| anyhow!("No root synced for {}", root_key))?;

        let index = leaves
            .iter()
            .position(|c| c.to_lowercase() == commitment.to_lowercase())
            .ok_or_else(|| anyhow!("Commitment {} not found in {} tree", commitment, chain_name))?;

        let proof = self.compute_merkle_proof(&leaves, index, 20)?;

        let local_root = self.verify_proof_generates_root(commitment, &proof, index, 20)?;

        if local_root.to_lowercase() != synced_root.to_lowercase() {
            return Err(anyhow!(
                "Root mismatch: Local {} != Synced {}",
                local_root,
                synced_root
            ));
        }

        Ok((proof, index as u32))
    }

    pub fn compute_merkle_proof_from_db(
        &self,
        tree_name: &str,
        index: usize,
    ) -> Result<Vec<String>> {
        let mut proof = Vec::new();
        let mut curr_index = index;

        for level in 0..self.tree_depth {
            let sibling_index = if curr_index % 2 == 0 {
                curr_index + 1
            } else {
                curr_index - 1
            };

            let sibling = match tree_name {
                "mantle_commitments" => self
                    .database
                    .get_mantle_commitment_node(level, sibling_index)?,
                "ethereum_commitments" => self
                    .database
                    .get_ethereum_commitment_node(level, sibling_index)?,
                "mantle_intents" => self.database.get_mantle_node(level, sibling_index)?,
                "ethereum_intents" => self
                    .database
                    .get_ethereum_intent_node(level, sibling_index)?,
                _ => return Err(anyhow!("Unsupported tree type: {}", tree_name)),
            }
            .unwrap_or_else(|| ZERO_LEAF.to_string());

            proof.push(sibling);
            curr_index /= 2;
        }

        Ok(proof)
    }

    fn compute_merkle_proof(
        &self,
        leaves: &[String],
        index: usize,
        depth: usize,
    ) -> Result<Vec<String>> {
        let mut proof = Vec::new();
        let mut current_level = leaves.to_vec();
        let mut current_index = index;

        for _ in 0..depth {
            if current_level.len() % 2 != 0 {
                current_level.push(ZERO_LEAF.to_string());
            }

            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            let sibling = if sibling_index < current_level.len() {
                current_level[sibling_index].clone()
            } else {
                ZERO_LEAF.to_string()
            };
            proof.push(sibling);

            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let left = &current_level[i];
                let right = &current_level[i + 1];
                next_level.push(self.hash_pair(left, right)?);
            }

            current_level = next_level;
            current_index /= 2;
        }

        Ok(proof)
    }

    fn verify_proof_generates_root(
        &self,
        leaf: &str,
        proof: &[String],
        index: usize,
        depth: usize,
    ) -> Result<String> {
        let mut current_hash = leaf.to_string();
        let mut current_index = index;

        for sibling in proof.iter().take(depth) {
            current_hash = self.hash_pair(&current_hash, sibling)?;
            current_index /= 2;
        }

        Ok(current_hash)
    }

    pub async fn get_mantle_intents_root(&self) -> Result<String> {
        self.database
            .get_latest_root("mantle_intents")?
            .ok_or_else(|| anyhow!("No Mantle intents root found"))
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

    pub async fn get_tree_sizes(&self) -> Result<(usize, usize, usize, usize)> {
        let mantle_intents = self.database.get_mantle_tree_size()?;
        let mantle_commitments = self.database.get_mantle_commitment_tree_size()?;
        let ethereum_fills = self.database.get_ethereum_fill_tree_size()?;
        let ethereum_commitments = self.database.get_ethereum_commitment_tree_size()?;

        Ok((
            mantle_intents,
            mantle_commitments,
            ethereum_fills,
            ethereum_commitments,
        ))
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
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

        let _ = db.delete_merkle_tree_by_name("mantle_intents");
        let _ = db.delete_merkle_tree_by_name("mantle_commitments");
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

        for (tree_name, depth) in &[
            ("mantle_intents", 20),
            ("mantle_commitments", 20),
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

        let incremental_root = mgr.compute_mantle_intents_root().unwrap();
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

        let root = mgr.compute_mantle_intents_root().unwrap();
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

        let root = mgr.compute_mantle_intents_root().unwrap();
        assert_ne!(root, ZERO_LEAF);
        assert_ne!(root, leaf);
    }

    #[tokio::test]
    #[serial]
    async fn test_tree_sizes() {
        let mgr = setup_test_manager().await;
        mgr.database.clear_mantle_tree().unwrap();
        mgr.database.clear_ethereum_tree("ethereum_fills").unwrap();

        let (mantle_intents, _, eth_fills, _) = mgr.get_tree_sizes().await.unwrap();
        assert_eq!(mantle_intents, 0);
        assert_eq!(eth_fills, 0);

        mgr.append_mantle_leaf(
            "0x1111111111111111111111111111111111111111111111111111111111111111",
        )
        .await
        .unwrap();

        let (mantle_intents, _, _, _) = mgr.get_tree_sizes().await.unwrap();
        assert_eq!(mantle_intents, 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_deterministic_roots() {
        let mgr = setup_test_manager().await;

        let leaves = vec![
            "0x1111111111111111111111111111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222222222222222222222222222",
        ];

        mgr.database.clear_mantle_tree().unwrap();
        mgr.database.clear_mantle_nodes().unwrap();

        for leaf in &leaves {
            mgr.append_mantle_leaf(leaf).await.unwrap();
        }
        let root1 = mgr.compute_mantle_intents_root().unwrap();

        mgr.rebuild_mantle_fills_tree().await.unwrap();
        let root2 = mgr.compute_mantle_intents_root().unwrap();

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
}
