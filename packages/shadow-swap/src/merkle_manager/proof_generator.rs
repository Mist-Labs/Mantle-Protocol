use anyhow::{Context, Result, anyhow};
use ethers::utils::keccak256;
use std::sync::Arc;
use tracing::{debug, info};

use crate::database::database::Database;

const ZERO_LEAF: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

pub struct MerkleProofGenerator {
    database: Arc<Database>,
}

impl MerkleProofGenerator {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Hash a pair of nodes (sorted order like Solidity)
    fn hash_pair(left: &str, right: &str) -> Result<String> {
        let left_bytes =
            hex::decode(left.trim_start_matches("0x")).context("Failed to decode left hash")?;
        let right_bytes =
            hex::decode(right.trim_start_matches("0x")).context("Failed to decode right hash")?;

        if left_bytes.len() != 32 || right_bytes.len() != 32 {
            return Err(anyhow!(
                "Invalid hash length: left={}, right={}",
                left_bytes.len(),
                right_bytes.len()
            ));
        }

        let (first, second) = if left_bytes < right_bytes {
            (&left_bytes, &right_bytes)
        } else {
            (&right_bytes, &left_bytes)
        };

        let combined = [first.as_slice(), second.as_slice()].concat();
        let hash = keccak256(&combined);

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

    /// Generate Merkle proof for a commitment - FIXED VERSION
    ///
    /// # Arguments
    /// * `chain` - Chain name ("mantle" or "ethereum")
    /// * `commitment` - The commitment hash to generate proof for
    /// * `limit` - The exact number of leaves that were synced on-chain
    ///             This MUST match the contract's tree state!
    pub fn generate_proof(
        &self,
        chain: &str,
        commitment: &str,
        limit: usize,
    ) -> Result<(Vec<String>, usize, String)> {
        info!(
            "📋 Generating proof for chain '{}', commitment={}, limit={}",
            chain,
            &commitment[..10],
            limit
        );

        let mut leaves = self
            .database
            .get_commitments_for_tree(chain, limit as i64)?;

        if leaves.is_empty() {
            return Err(anyhow!(
                "No commitments found for chain '{}' with limit {}",
                chain,
                limit
            ));
        }

        // Find commitment index BEFORE padding
        let leaf_index = leaves
            .iter()
            .position(|c| c.to_lowercase() == commitment.to_lowercase())
            .ok_or_else(|| {
                anyhow!(
                    "Commitment {} not found in first {} leaves for chain '{}'",
                    &commitment[..10],
                    limit,
                    chain
                )
            })?;

        info!(
            "🔍 Found commitment at index {} (tree has {} leaves)",
            leaf_index,
            leaves.len()
        );

        let tree_size = std::cmp::max(2, Self::next_power_of_2(leaves.len()));
        leaves.resize(tree_size, ZERO_LEAF.to_string());

        let height = (tree_size as f64).log2() as usize;

        info!("🌳 Tree size: {} (min 2), height: {}", tree_size, height);

        let mut layer = leaves;
        let mut proof = Vec::with_capacity(height);
        let mut current_index = leaf_index;

        for level in 0..height {
            let sibling_index = current_index ^ 1;
            proof.push(layer[sibling_index].clone());

            debug!(
                "  Level {}: index={}, sibling={}",
                level,
                current_index,
                &layer[sibling_index][..10]
            );

            let mut next_layer = Vec::with_capacity(layer.len() / 2);
            for i in 0..(layer.len() / 2) {
                next_layer.push(Self::hash_pair(&layer[2 * i], &layer[2 * i + 1])?);
            }

            layer = next_layer;
            current_index /= 2;
        }

        let root = layer[0].clone();

        info!(
            "✅ Proof generated: {} siblings, root={}",
            proof.len(),
            &root[..10]
        );

        Ok((proof, leaf_index, root))
    }

    pub fn compute_root(&self, chain: &str) -> Result<String> {
        let leaves = self.database.get_all_commitments_for_chain(chain)?;

        if leaves.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        let tree_size = std::cmp::max(2, Self::next_power_of_2(leaves.len()));
        let mut layer: Vec<String> = leaves;
        layer.resize(tree_size, ZERO_LEAF.to_string());

        while layer.len() > 1 {
            let mut next_layer = Vec::with_capacity(layer.len() / 2);
            for i in 0..(layer.len() / 2) {
                next_layer.push(Self::hash_pair(&layer[2 * i], &layer[2 * i + 1])?);
            }
            layer = next_layer;
        }

        Ok(layer[0].clone())
    }

    /// Verify a Merkle proof
    pub fn verify_proof(
        &self,
        proof: &[String],
        root: &str,
        leaf: &str,
        index: usize,
    ) -> Result<bool> {
        let mut computed_hash = leaf.to_string();
        let mut current_index = index;

        debug!("🔍 Verifying proof:");
        debug!("  Leaf: {}", &leaf[..10]);
        debug!("  Index: {}", index);
        debug!("  Expected root: {}", &root[..10]);

        for (level, proof_element) in proof.iter().enumerate() {
            let is_right = (current_index & 1) == 1;

            computed_hash = if is_right {
                Self::hash_pair(proof_element, &computed_hash)?
            } else {
                Self::hash_pair(&computed_hash, proof_element)?
            };

            debug!(
                "  Level {}: {} + sibling → {}",
                level,
                if is_right { "sibling" } else { "hash" },
                &computed_hash[..10]
            );

            current_index >>= 1;
        }

        debug!("  Computed root: {}", &computed_hash[..10]);

        let is_valid = computed_hash.to_lowercase() == root.to_lowercase();

        if is_valid {
            info!("✅ Proof verification successful");
        } else {
            info!("❌ Proof verification failed");
        }

        Ok(is_valid)
    }

    // ============   FILL PROOF    ===============
    /// Generate Merkle proof for a fill (intent_id in fill tree)
    ///
    /// # Arguments
    /// * `chain` - Chain name ("mantle" or "ethereum")
    /// * `intent_id` - The intent ID to generate proof for
    /// * `limit` - The exact number of fills that were synced on-chain
    pub fn generate_fill_proof(
        &self,
        chain: &str,
        intent_id: &str,
        limit: usize,
    ) -> Result<(Vec<String>, usize, String)> {
        info!(
            "📋 Generating fill proof for chain '{}', intent_id={}, limit={}",
            chain,
            &intent_id[..10],
            limit
        );

        let mut fills = self.database.get_fills_for_tree(chain, limit as i64)?;

        if fills.is_empty() {
            return Err(anyhow!(
                "No fills found for chain '{}' with limit {}",
                chain,
                limit
            ));
        }

        let fill_index = fills
            .iter()
            .position(|f| f.to_lowercase() == intent_id.to_lowercase())
            .ok_or_else(|| {
                anyhow!(
                    "Intent ID {} not found in first {} fills for chain '{}'",
                    &intent_id[..10],
                    limit,
                    chain
                )
            })?;

        info!(
            "🔍 Found intent_id at index {} (tree has {} fills)",
            fill_index,
            fills.len()
        );

        let tree_size = std::cmp::max(2, Self::next_power_of_2(fills.len()));
        fills.resize(tree_size, ZERO_LEAF.to_string());

        let height = (tree_size as f64).log2() as usize;

        info!(
            "🌳 Fill tree size: {} (min 2), height: {}",
            tree_size, height
        );

        let mut layer = fills;
        let mut proof = Vec::with_capacity(height);
        let mut current_index = fill_index;

        for level in 0..height {
            let sibling_index = current_index ^ 1;
            proof.push(layer[sibling_index].clone());

            debug!(
                "  Level {}: index={}, sibling={}",
                level,
                current_index,
                &layer[sibling_index][..10]
            );

            let mut next_layer = Vec::with_capacity(layer.len() / 2);
            for i in 0..(layer.len() / 2) {
                next_layer.push(Self::hash_pair(&layer[2 * i], &layer[2 * i + 1])?);
            }

            layer = next_layer;
            current_index /= 2;
        }

        let root = layer[0].clone();

        info!(
            "✅ Fill proof generated: {} siblings, root={}",
            proof.len(),
            &root[..10]
        );

        Ok((proof, fill_index, root))
    }

    pub fn compute_fill_root(&self, chain: &str) -> Result<String> {
        let fills = self.database.get_all_fills_for_chain(chain)?;

        if fills.is_empty() {
            return Ok(ZERO_LEAF.to_string());
        }

        let tree_size = std::cmp::max(2, Self::next_power_of_2(fills.len()));
        let mut layer: Vec<String> = fills;
        layer.resize(tree_size, ZERO_LEAF.to_string());

        while layer.len() > 1 {
            let mut next_layer = Vec::with_capacity(layer.len() / 2);
            for i in 0..(layer.len() / 2) {
                next_layer.push(Self::hash_pair(&layer[2 * i], &layer[2 * i + 1])?);
            }
            layer = next_layer;
        }

        Ok(layer[0].clone())
    }

    /// Get Ethereum proof
    pub fn get_ethereum_proof(
        &self,
        commitment: &str,
        limit: usize,
    ) -> Result<(Vec<String>, usize, String)> {
        self.generate_proof("ethereum", commitment, limit)
    }

    /// Get Mantle proof
    pub fn get_mantle_proof(
        &self,
        commitment: &str,
        limit: usize,
    ) -> Result<(Vec<String>, usize, String)> {
        self.generate_proof("mantle", commitment, limit)
    }

    /// Compute Ethereum root
    pub fn compute_ethereum_root(&self) -> Result<String> {
        self.compute_root("ethereum")
    }

    /// Compute Mantle root
    pub fn compute_mantle_root(&self) -> Result<String> {
        self.compute_root("mantle")
    }

    pub fn get_ethereum_fill_proof(
        &self,
        intent_id: &str,
        limit: usize,
    ) -> Result<(Vec<String>, usize, String)> {
        self.generate_fill_proof("ethereum", intent_id, limit)
    }

    /// Get Mantle fill proof
    pub fn get_mantle_fill_proof(
        &self,
        intent_id: &str,
        limit: usize,
    ) -> Result<(Vec<String>, usize, String)> {
        self.generate_fill_proof("mantle", intent_id, limit)
    }

    /// Compute Ethereum fill root
    pub fn compute_ethereum_fill_root(&self) -> Result<String> {
        self.compute_fill_root("ethereum")
    }

    /// Compute Mantle fill root
    pub fn compute_mantle_fill_root(&self) -> Result<String> {
        self.compute_fill_root("mantle")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_pair_sorted() {
        let a = "0x2222222222222222222222222222222222222222222222222222222222222222";
        let b = "0x1111111111111111111111111111111111111111111111111111111111111111";

        let result1 = MerkleProofGenerator::hash_pair(a, b).unwrap();
        let result2 = MerkleProofGenerator::hash_pair(b, a).unwrap();

        assert_eq!(result1, result2, "Hash pair should be commutative (sorted)");
    }

    #[test]
    fn test_next_power_of_2() {
        assert_eq!(MerkleProofGenerator::next_power_of_2(0), 1);
        assert_eq!(MerkleProofGenerator::next_power_of_2(1), 1);
        assert_eq!(MerkleProofGenerator::next_power_of_2(2), 2);
        assert_eq!(MerkleProofGenerator::next_power_of_2(3), 4);
        assert_eq!(MerkleProofGenerator::next_power_of_2(5), 8);
        assert_eq!(MerkleProofGenerator::next_power_of_2(9), 16);
    }

    #[test]
    fn test_hash_pair_matches_solidity() {
        let a = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let b = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        let result = MerkleProofGenerator::hash_pair(a, b).unwrap();

        // Verify sorted order
        let a_bytes = hex::decode(&a[2..]).unwrap();
        let b_bytes = hex::decode(&b[2..]).unwrap();
        let mut combined = Vec::new();
        combined.extend_from_slice(&a_bytes);
        combined.extend_from_slice(&b_bytes);
        let expected = format!("0x{}", hex::encode(keccak256(&combined)));

        assert_eq!(result, expected);
    }

    #[test]
    fn test_single_leaf_tree() {
        let leaf = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let tree_size = MerkleProofGenerator::next_power_of_2(1);
        assert_eq!(tree_size, 1);
    }

    #[test]
    fn test_two_leaf_tree() {
        let leaf0 = "0x1111111111111111111111111111111111111111111111111111111111111111";
        let leaf1 = "0x2222222222222222222222222222222222222222222222222222222222222222";

        let expected_root = MerkleProofGenerator::hash_pair(leaf0, leaf1).unwrap();
        assert!(expected_root.starts_with("0x"));
        assert_eq!(expected_root.len(), 66);
    }

    #[test]
    fn test_proof_verification() {
        // Simple 2-leaf tree
        let leaf0 = "0x1111111111111111111111111111111111111111111111111111111111111111";
        let leaf1 = "0x2222222222222222222222222222222222222222222222222222222222222222";

        let root = MerkleProofGenerator::hash_pair(leaf0, leaf1).unwrap();
        let proof = vec![leaf1.to_string()];

        // Manual verification
        let mut computed = leaf0.to_string();
        for proof_element in &proof {
            computed = MerkleProofGenerator::hash_pair(&computed, proof_element).unwrap();
        }

        assert_eq!(computed.to_lowercase(), root.to_lowercase());
    }

    #[test]
    fn test_invalid_hash_length() {
        let a = "0x1111";
        let b = "0x2222222222222222222222222222222222222222222222222222222222222222";

        let result = MerkleProofGenerator::hash_pair(a, b);
        assert!(result.is_err());
    }
}
