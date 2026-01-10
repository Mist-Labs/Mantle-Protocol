use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{error, info};

use crate::{
    database::database::Database,
    relay_coordinator::model::{EthereumRelayer, MantleRelayer},
};

const MANTLE_CHAIN_ID: u32 = 5003;
const ETHEREUM_CHAIN_ID: u32 = 11155111;
const ZERO_LEAF: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

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
        let results = tokio::join!(
            self.sync_ethereum_commitments_to_mantle(),
            self.sync_mantle_fills_to_ethereum(),
            self.sync_mantle_commitments_to_ethereum(),
            self.sync_ethereum_fills_to_mantle()
        );

        if let Err(e) = results.0 {
            error!("❌ Eth-Commit → Mantle: {}", e);
        }
        if let Err(e) = results.1 {
            error!("❌ Mantle-Fill → Eth: {}", e);
        }
        if let Err(e) = results.2 {
            error!("❌ Mantle-Commit → Eth: {}", e);
        }
        if let Err(e) = results.3 {
            error!("❌ Eth-Fill → Mantle: {}", e);
        }

        Ok(())
    }

    pub async fn sync_ethereum_commitments_to_mantle(&self) -> Result<()> {
        let db_root = self.get_db_root_standardized("ethereum_commitments")?;
        let onchain_root = self.mantle_relayer.get_synced_ethereum_commitment_root().await?.to_lowercase();

        if db_root != onchain_root {
            info!("🌉 [ETH → MANTLE] Syncing commitment root: {}", &db_root[..10]);
            let root_bytes = self.hex_to_bytes32(&db_root)?;
            self.mantle_relayer.sync_source_chain_commitment_root_tx(ETHEREUM_CHAIN_ID, root_bytes).await?;
            info!("✅ Commitment root synced");
        }

        Ok(())
    }

    async fn sync_mantle_fills_to_ethereum(&self) -> Result<()> {
        let db_root = self.get_db_root_standardized("mantle_fills")?;
        if db_root == ZERO_LEAF {
            return Ok(());
        }

        let onchain_root = self.ethereum_relayer.get_synced_mantle_fill_root().await?.to_lowercase();

        if db_root != onchain_root {
            info!("🌉 [MANTLE → ETH] Syncing fill root: {}", &db_root[..10]);
            let root_bytes = self.hex_to_bytes32(&db_root)?;
            self.ethereum_relayer.sync_dest_chain_fill_root_tx(MANTLE_CHAIN_ID, root_bytes).await?;
            info!("✅ Fill root synced");
        }

        Ok(())
    }

    pub async fn sync_mantle_commitments_to_ethereum(&self) -> Result<()> {
        let db_root = self.get_db_root_standardized("mantle_commitments")?;
        let onchain_root = self.ethereum_relayer.get_synced_mantle_commitment_root().await?.to_lowercase();

        if db_root != onchain_root {
            info!("🌉 [MANTLE → ETH] Syncing commitment root: {}", &db_root[..10]);
            let root_bytes = self.hex_to_bytes32(&db_root)?;
            self.ethereum_relayer.sync_source_chain_commitment_root_tx(MANTLE_CHAIN_ID, root_bytes).await?;
            info!("✅ Commitment root synced");
        }

        Ok(())
    }

    async fn sync_ethereum_fills_to_mantle(&self) -> Result<()> {
        let db_root = self.get_db_root_standardized("ethereum_fills")?;
        if db_root == ZERO_LEAF {
            return Ok(());
        }

        let onchain_root = self.mantle_relayer.get_synced_ethereum_fill_root().await?.to_lowercase();

        if db_root != onchain_root {
            info!("🌉 [ETH → MANTLE] Syncing fill root: {}", &db_root[..10]);
            let root_bytes = self.hex_to_bytes32(&db_root)?;
            self.mantle_relayer.sync_dest_chain_fill_root_tx(ETHEREUM_CHAIN_ID, root_bytes).await?;
            info!("✅ Fill root synced");
        }

        Ok(())
    }

    fn get_db_root_standardized(&self, tree_name: &str) -> Result<String> {
        let root = self.db.get_latest_root(tree_name)?.unwrap_or_else(|| ZERO_LEAF.to_string());
        let cleaned = if root.starts_with("0x") {
            root.to_lowercase()
        } else {
            format!("0x{}", root.to_lowercase())
        };
        Ok(cleaned)
    }

    fn hex_to_bytes32(&self, hex_str: &str) -> Result<[u8; 32]> {
        let s = hex_str.trim_start_matches("0x");
        let vec = hex::decode(s).map_err(|e| anyhow!("Invalid hex format: {}", e))?;
        vec.try_into().map_err(|_| anyhow!("Hex string must be exactly 32 bytes"))
    }

    pub async fn run(self: Arc<Self>) {
        info!("🔄 RootSyncCoordinator started ({}s interval)", self.sync_interval_secs);
        loop {
            let _ = self.sync_all_roots().await;
            sleep(Duration::from_secs(self.sync_interval_secs)).await;
        }
    }

    pub async fn sync_now(&self) -> Result<()> {
        self.sync_all_roots().await
    }
}