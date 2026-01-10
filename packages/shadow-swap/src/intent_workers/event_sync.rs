use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::{
    database::database::Database,
    merkle_manager::merkle_manager::MerkleTreeManager,
    relay_coordinator::model::{EthereumRelayer, MantleRelayer},
};

pub struct IntentSyncService {
    database: Arc<Database>,
    mantle_relayer: Arc<MantleRelayer>,
    ethereum_relayer: Arc<EthereumRelayer>,
    merkle_manager: Arc<MerkleTreeManager>,
}

impl IntentSyncService {
    pub fn new(
        database: Arc<Database>,
        mantle_relayer: Arc<MantleRelayer>,
        ethereum_relayer: Arc<EthereumRelayer>,
        merkle_manager: Arc<MerkleTreeManager>,
    ) -> Self {
        Self {
            database,
            mantle_relayer,
            ethereum_relayer,
            merkle_manager,
        }
    }

    pub async fn resync_ethereum_intents(
        &self,
        from_block: u64,
        clear_existing: bool,
    ) -> Result<()> {
        info!("🔄 Starting Ethereum resync from block {}", from_block);

        if clear_existing {
            warn!("⚠️  Clearing existing Ethereum intents to fix metadata/ordering");
            self.database.clear_all_intents_for_chain("ethereum")?;
        }

        // The relayer now uses the corrected 160-byte data length check
        let events = self
            .ethereum_relayer
            .fetch_all_intent_created_events(from_block)
            .await?;

        info!("📥 Processing {} events for Ethereum", events.len());

        for (idx, event) in events.iter().enumerate() {
            // CRITICAL: Ensure we aren't inserting nulls that break Merkle ordering
            if event.block_number.is_none() || event.log_index.is_none() {
                error!(
                    "❌ Event {} is missing critical ordering metadata!",
                    event.intent_id
                );
                return Err(anyhow!(
                    "Cannot sync events without block_number and log_index"
                ));
            }

            if idx % 100 == 0 {
                info!("  Progress: {}/{}", idx, events.len());
            }

            // This now includes the block_number and log_index for the ORDER BY clause
            self.database.upsert_intent_from_event(event, "ethereum")?;
        }

        info!("✅ Rebuilding Ethereum Merkle tree with deterministic ordering");
        self.merkle_manager
            .rebuild_ethereum_commitments_tree()
            .await?;

        let db_root = self.merkle_manager.compute_ethereum_commitments_root()?;
        let onchain_root = self.ethereum_relayer.get_intent_pool_root().await?;

        info!("🔍 Ethereum Verification:");
        info!("  DB root:       {}", db_root);
        info!("  On-chain root: {}", onchain_root);

        if db_root.to_lowercase() == onchain_root.to_lowercase() {
            info!("✅ SUCCESS! Ethereum roots match perfectly!");
            Ok(())
        } else {
            warn!("❌ Root mismatch. This usually means an event was missed or ordering is wrong.");
            Err(anyhow!("Ethereum Root Mismatch"))
        }
    }

    pub async fn resync_mantle_intents(&self, from_block: u64, clear_existing: bool) -> Result<()> {
        info!("🔄 Starting Mantle resync from block {}", from_block);

        if clear_existing {
            warn!("⚠️  Clearing existing Mantle intents");
            self.database.clear_all_intents_for_chain("mantle")?;
        }

        let events = self
            .mantle_relayer
            .fetch_all_intent_created_events(from_block)
            .await?;

        info!("📥 Processing {} events for Mantle", events.len());

        for (idx, event) in events.iter().enumerate() {
            if event.block_number.is_none() {
                error!("❌ Mantle Event {} missing block_number", event.intent_id);
            }

            if idx % 100 == 0 {
                info!("  Progress: {}/{}", idx, events.len());
            }
            self.database.upsert_intent_from_event(event, "mantle")?;
        }

        info!("✅ Rebuilding Mantle Merkle tree");
        self.merkle_manager
            .rebuild_mantle_commitments_tree()
            .await?;

        let db_root = self.merkle_manager.compute_mantle_commitments_root()?;
        let onchain_root = self.mantle_relayer.get_intent_pool_root().await?;

        info!("🔍 Mantle Verification:");
        info!("  DB root:       {}", db_root);
        info!("  On-chain root: {}", onchain_root);

        if db_root.to_lowercase() == onchain_root.to_lowercase() {
            info!("✅ SUCCESS! Mantle roots match!");
            Ok(())
        } else {
            Err(anyhow!("❌ Mantle Root Mismatch"))
        }
    }

    pub async fn verify_sync_status(&self) -> Result<()> {
        info!("🔍 Verifying sync status for all chains");

        info!("\n=== MANTLE ===");
        let mantle_events = self
            .mantle_relayer
            .fetch_all_intent_created_events(33091000)
            .await?;
        let mantle_db_count = self.database.get_all_commitments_for_chain("mantle")?.len();
        let mantle_onchain_count = mantle_events.len();
        let mantle_db_root = self.merkle_manager.compute_mantle_commitments_root()?;
        let mantle_onchain_root = self.mantle_relayer.get_intent_pool_root().await?;

        info!("  DB commitments:    {}", mantle_db_count);
        info!("  On-chain events:   {}", mantle_onchain_count);
        info!("  DB root:           {}", mantle_db_root);
        info!("  On-chain root:     {}", mantle_onchain_root);

        if mantle_db_count != mantle_onchain_count {
            warn!(
                "  ❌ Count mismatch! Missing {} events",
                mantle_onchain_count as i64 - mantle_db_count as i64
            );
        }
        if mantle_db_root.to_lowercase() != mantle_onchain_root.to_lowercase() {
            warn!("  ❌ Root mismatch!");
        }

        info!("\n=== ETHEREUM ===");
        let eth_events = self
            .ethereum_relayer
            .fetch_all_intent_created_events(9993815)
            .await?;
        let eth_db_count = self
            .database
            .get_all_commitments_for_chain("ethereum")?
            .len();
        let eth_onchain_count = eth_events.len();
        let eth_db_root = self.merkle_manager.compute_ethereum_commitments_root()?;
        let eth_onchain_root = self.ethereum_relayer.get_intent_pool_root().await?;

        info!("  DB commitments:    {}", eth_db_count);
        info!("  On-chain events:   {}", eth_onchain_count);
        info!("  DB root:           {}", eth_db_root);
        info!("  On-chain root:     {}", eth_onchain_root);

        if eth_db_count != eth_onchain_count {
            warn!(
                "  ❌ Count mismatch! Missing {} events",
                eth_onchain_count as i64 - eth_db_count as i64
            );
        }
        if eth_db_root.to_lowercase() != eth_onchain_root.to_lowercase() {
            warn!("  ❌ Root mismatch!");
        }

        Ok(())
    }
}
