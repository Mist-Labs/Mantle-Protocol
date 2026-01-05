use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

use crate::{
    database::database::Database,
    relay_coordinator::model::{EthereumRelayer, MantleRelayer},
};

const MANTLE_CHAIN_ID: u32 = 5003;
const ETHEREUM_CHAIN_ID: u32 = 11155111;

pub struct RootSyncCoordinator {
    db: Arc<Database>,
    ethereum_relayer: Arc<EthereumRelayer>,
    mantle_relayer: Arc<MantleRelayer>,
    sync_interval_secs: u64,
}

impl RootSyncCoordinator {
    pub fn new(
        db: Arc<Database>,
        ethereum_relayer: Arc<EthereumRelayer>,
        mantle_relayer: Arc<MantleRelayer>,
        sync_interval_secs: u64,
    ) -> Self {
        Self {
            db,
            ethereum_relayer,
            mantle_relayer,
            sync_interval_secs,
        }
    }

    pub async fn sync_all_roots(&self) -> Result<()> {
        info!("🔄 Starting complete 4-way root sync");

        if let Err(e) = self.sync_ethereum_commitments_to_mantle().await {
            error!("❌ Failed Ethereum→Mantle commitment sync: {}", e);
        }

        if let Err(e) = self.sync_mantle_fills_to_ethereum().await {
            error!("❌ Failed Mantle→Ethereum fill sync: {}", e);
        }

        if let Err(e) = self.sync_mantle_commitments_to_ethereum().await {
            error!("❌ Failed Mantle→Ethereum commitment sync: {}", e);
        }

        if let Err(e) = self.sync_ethereum_fills_to_mantle().await {
            error!("❌ Failed Ethereum→Mantle fill sync: {}", e);
        }

        info!("✅ 4-way root sync completed");
        Ok(())
    }

    async fn sync_ethereum_commitments_to_mantle(&self) -> Result<()> {
        debug!("🔍 Syncing Ethereum commitment root → Mantle Settlement");

        let offchain_root = self
            .db
            .get_latest_root("ethereum_commitments")?
            .ok_or_else(|| anyhow!("No Ethereum commitment root"))?;

        info!("📊 Ethereum commitment root: {}", offchain_root);

        let last_synced = self
            .db
            .get_last_synced_root_by_type("ethereum_commitments_to_mantle_settlement")?;

        if last_synced.as_deref() == Some(&offchain_root) {
            debug!("✅ Already synced");
            return Ok(());
        }

        info!("🌳 Syncing Ethereum commitments → Mantle Settlement");
        let tx_hash = self
            .mantle_relayer
            .sync_source_root_tx(ETHEREUM_CHAIN_ID, offchain_root.clone())
            .await?;

        self.db.record_root_sync(
            "ethereum_commitments_to_mantle_settlement",
            &offchain_root,
            &tx_hash,
        )?;

        info!("✅ Synced! Tx: {}", tx_hash);
        Ok(())
    }

    async fn sync_mantle_fills_to_ethereum(&self) -> Result<()> {
        debug!("🔍 Syncing Mantle fill root → Ethereum IntentPool");

        let mantle_fill_root = self.mantle_relayer.get_fill_root().await?;

        info!("📊 Mantle fill root: {}", mantle_fill_root);

        let last_synced = self
            .db
            .get_last_synced_root_by_type("mantle_fills_to_ethereum_intentpool")?;

        if last_synced.as_deref() == Some(&mantle_fill_root) {
            debug!("✅ Already synced");
            return Ok(());
        }

        let root_bytes: [u8; 32] = hex::decode(&mantle_fill_root[2..])
            .map_err(|e| anyhow!("Invalid hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid length"))?;

        info!("🌳 Syncing Mantle fills → Ethereum IntentPool");
        let tx_hash = self
            .ethereum_relayer
            .sync_dest_root_tx(MANTLE_CHAIN_ID, root_bytes)
            .await?;

        self.db.record_root_sync(
            "mantle_fills_to_ethereum_intentpool",
            &mantle_fill_root,
            &tx_hash,
        )?;

        info!("✅ Synced! Tx: {}", tx_hash);
        Ok(())
    }

    async fn sync_mantle_commitments_to_ethereum(&self) -> Result<()> {
        debug!("🔍 Syncing Mantle commitment root → Ethereum Settlement");

        let offchain_root = self
            .db
            .get_latest_root("mantle")?
            .ok_or_else(|| anyhow!("No Mantle commitment root"))?;

        info!("📊 Mantle commitment root: {}", offchain_root);

        let last_synced = self
            .db
            .get_last_synced_root_by_type("mantle_commitments_to_ethereum_settlement")?;

        if last_synced.as_deref() == Some(&offchain_root) {
            debug!("✅ Already synced");
            return Ok(());
        }

        info!("🌳 Syncing Mantle commitments → Ethereum Settlement");
        let tx_hash = self
            .ethereum_relayer
            .sync_source_root_tx(MANTLE_CHAIN_ID, offchain_root.clone())
            .await?;

        self.db.record_root_sync(
            "mantle_commitments_to_ethereum_settlement",
            &offchain_root,
            &tx_hash,
        )?;

        info!("✅ Synced! Tx: {}", tx_hash);
        Ok(())
    }

    async fn sync_ethereum_fills_to_mantle(&self) -> Result<()> {
        debug!("🔍 Syncing Ethereum fill root → Mantle IntentPool");

        let ethereum_fill_root = self.ethereum_relayer.get_fill_root().await?;

        info!("📊 Ethereum fill root: {}", ethereum_fill_root);

        let last_synced = self
            .db
            .get_last_synced_root_by_type("ethereum_fills_to_mantle_intentpool")?;

        if last_synced.as_deref() == Some(&ethereum_fill_root) {
            debug!("✅ Already synced");
            return Ok(());
        }

        let root_bytes: [u8; 32] = hex::decode(&ethereum_fill_root[2..])
            .map_err(|e| anyhow!("Invalid hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid length"))?;

        info!("🌳 Syncing Ethereum fills → Mantle IntentPool");
        let tx_hash = self
            .mantle_relayer
            .sync_dest_root_tx(ETHEREUM_CHAIN_ID, root_bytes)
            .await?;

        self.db.record_root_sync(
            "ethereum_fills_to_mantle_intentpool",
            &ethereum_fill_root,
            &tx_hash,
        )?;

        info!("✅ Synced! Tx: {}", tx_hash);
        Ok(())
    }

    pub async fn run(self: Arc<Self>) {
        info!(
            "🚀 Starting root sync coordinator (interval: {}s)",
            self.sync_interval_secs
        );

        loop {
            if let Err(e) = self.sync_all_roots().await {
                error!("❌ Root sync failed: {:?}", e);
            }

            sleep(Duration::from_secs(self.sync_interval_secs)).await;
        }
    }

    pub async fn sync_now(&self) -> Result<()> {
        info!("🔧 Manual root sync triggered");
        self.sync_all_roots().await
    }
}
