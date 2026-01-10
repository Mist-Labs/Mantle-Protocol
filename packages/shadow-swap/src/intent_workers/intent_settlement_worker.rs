use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

use crate::{
    database::database::Database,
    models::model::{Intent, IntentStatus},
    relay_coordinator::model::{BridgeCoordinator, EthereumRelayer, MantleRelayer},
};

const ETHEREUM_CHAIN_ID: u32 = 11155111;
const MANTLE_CHAIN_ID: u32 = 5003;
const MAX_CONCURRENT_SETTLEMENTS: usize = 3;
const ZERO_LEAF: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

pub struct IntentSettlementWorker {
    database: Arc<Database>,
    mantle_relayer: Arc<MantleRelayer>,
    ethereum_relayer: Arc<EthereumRelayer>,
    coordinator: Arc<BridgeCoordinator>,
    poll_interval: Duration,
}

impl IntentSettlementWorker {
    pub fn new(
        database: Arc<Database>,
        mantle_relayer: Arc<MantleRelayer>,
        ethereum_relayer: Arc<EthereumRelayer>,
        coordinator: Arc<BridgeCoordinator>,
    ) -> Self {
        Self {
            database,
            mantle_relayer,
            ethereum_relayer,
            coordinator,
            poll_interval: Duration::from_secs(10),
        }
    }

    pub async fn run(&self) {
        info!("🔄 Intent settlement worker started");
        loop {
            if let Err(e) = self.process_pending_settlements().await {
                error!("Settlement worker error: {}", e);
            }
            sleep(self.poll_interval).await;
        }
    }

    async fn process_pending_settlements(&self) -> Result<()> {
        let filled_intents = self.database.get_intents_by_status(IntentStatus::Filled)?;

        if filled_intents.is_empty() {
            return Ok(());
        }

        info!(
            "📋 Found {} intents pending settlement",
            filled_intents.len()
        );

        let mut tasks = Vec::new();
        for intent in filled_intents.into_iter().take(MAX_CONCURRENT_SETTLEMENTS) {
            let worker = self.clone_for_task();
            let task = tokio::spawn(async move {
                let intent_id = intent.id.clone();
                match worker.process_single_settlement(&intent).await {
                    Ok(_) => info!("✅ Settled intent {}", &intent_id[..10]),
                    Err(e) => error!("❌ Failed to settle intent {}: {:#?}", &intent_id[..10], e),
                }
            });
            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }
        Ok(())
    }

    async fn process_single_settlement(&self, intent: &Intent) -> Result<()> {
        info!("⚙️ Processing settlement for intent {}", &intent.id[..10]);

        let (source_chain, dest_chain, dest_chain_id) = match intent.source_chain.as_str() {
            "ethereum" => ("ethereum", "mantle", MANTLE_CHAIN_ID),
            "mantle" => ("mantle", "ethereum", ETHEREUM_CHAIN_ID),
            chain => return Err(anyhow!("Unsupported source chain: {}", chain)),
        };

        let is_filled = match dest_chain_id {
            ETHEREUM_CHAIN_ID => {
                self.ethereum_relayer
                    .check_intent_filled(&intent.id)
                    .await?
            }
            MANTLE_CHAIN_ID => self.mantle_relayer.check_intent_filled(&intent.id).await?,
            _ => unreachable!(),
        };

        if !is_filled {
            warn!("⚠️ Intent {} not filled on-chain yet", &intent.id[..10]);
            return Ok(());
        }

        let dest_fill_root = self
            .wait_for_db_sync_with_fill_tree(
                source_chain,
                dest_chain,
                dest_chain_id,
                &format!("{}_fills", dest_chain),
                Duration::from_secs(60),
            )
            .await?;

        info!("   Destination fill root: {}", &dest_fill_root[..18]);

        let sync_result = tokio::time::timeout(
            Duration::from_secs(120),
            self.ensure_fill_root_synced_to_source(source_chain, dest_chain_id, &dest_fill_root),
        )
        .await;

        match sync_result {
            Ok(Ok(())) => info!("   ✅ Fill root synced to source chain"),
            Ok(Err(e)) => return Err(anyhow!("Fill root sync failed: {}", e)),
            Err(_) => return Err(anyhow!("Fill root sync timeout after 2min")),
        }

        info!("   Generating fill proof...");
        let (fill_proof, leaf_index) = self.get_fill_proof(&intent.id, dest_chain_id).await?;

        info!(
            "   Proof generated - Length: {}, Index: {}",
            fill_proof.len(),
            leaf_index
        );

        let solver_address = intent
            .solver_address
            .as_ref()
            .ok_or_else(|| anyhow!("Missing solver address"))?;

        let tx_hash = match source_chain {
            "ethereum" => {
                self.ethereum_relayer
                    .settle_intent(&intent.id, solver_address, &fill_proof, leaf_index)
                    .await?
            }
            "mantle" => {
                self.mantle_relayer
                    .settle_intent(&intent.id, solver_address, &fill_proof, leaf_index)
                    .await?
            }
            _ => unreachable!(),
        };

        self.database
            .update_source_settlement_txid(&intent.id, &tx_hash)?;
        self.database
            .update_intent_status(&intent.id, IntentStatus::SolverPaid)?;

        info!("🎉 Intent {} settled: {}", &intent.id[..10], tx_hash);

        tokio::spawn({
            let coordinator = self.coordinator.clone();
            let intent_id = intent.id.clone();
            async move {
                // Small delay to ensure DB transaction committed
                tokio::time::sleep(Duration::from_millis(500)).await;

                match coordinator.database.get_intent_by_id(&intent_id) {
                    Ok(Some(intent)) => {
                        info!("🔄 Auto-claiming for intent {}", &intent_id[..10]);
                        if let Err(e) = coordinator.claim_for_user(&intent).await {
                            error!("❌ Auto-claim failed for {}: {}", &intent_id[..10], e);
                        }
                    }
                    Ok(None) => error!("Intent {} not found for auto-claim", &intent_id[..10]),
                    Err(e) => error!("DB error during auto-claim for {}: {}", &intent_id[..10], e),
                }
            }
        });

        Ok(())
    }

    async fn wait_for_db_sync_with_fill_tree(
        &self,
        source_chain: &str,
        dest_chain: &str,
        dest_chain_id: u32,
        tree_name: &str,
        timeout: Duration,
    ) -> Result<String> {
        let start = tokio::time::Instant::now();

        info!("⏳ Waiting for DB to sync with {} fill tree...", dest_chain);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!("Timeout waiting for DB fill tree sync"));
            }

            let onchain_fill_root = match (source_chain, dest_chain_id) {
                ("mantle", ETHEREUM_CHAIN_ID) => {
                    self.mantle_relayer.get_synced_ethereum_fill_root().await?
                }
                ("ethereum", MANTLE_CHAIN_ID) => {
                    self.ethereum_relayer.get_synced_mantle_fill_root().await?
                }
                _ => return Err(anyhow!("Invalid chain combination")),
            };

            let db_root = self
                .database
                .get_latest_root(tree_name)?
                .unwrap_or_else(|| ZERO_LEAF.to_string());

            let db_root_normalized = if db_root.starts_with("0x") {
                db_root.to_lowercase()
            } else {
                format!("0x{}", db_root.to_lowercase())
            };

            info!(
                "   Synced on {}: {} | DB fill: {}",
                source_chain,
                &onchain_fill_root[..18],
                &db_root_normalized[..18]
            );

            if onchain_fill_root.to_lowercase() == db_root_normalized {
                info!("   ✅ DB synced with on-chain fill tree");
                return Ok(onchain_fill_root);
            }

            info!("   ⏳ DB fill tree not synced yet, waiting 2s...");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    async fn ensure_fill_root_synced_to_source(
        &self,
        source_chain: &str,
        dest_chain_id: u32,
        expected_fill_root: &str,
    ) -> Result<()> {
        let synced_root = match source_chain {
            "ethereum" => {
                if dest_chain_id == MANTLE_CHAIN_ID {
                    self.ethereum_relayer.get_synced_mantle_fill_root().await?
                } else {
                    return Err(anyhow!("Invalid chain combination"));
                }
            }
            "mantle" => {
                if dest_chain_id == ETHEREUM_CHAIN_ID {
                    self.mantle_relayer.get_synced_ethereum_fill_root().await?
                } else {
                    return Err(anyhow!("Invalid chain combination"));
                }
            }
            _ => return Err(anyhow!("Unknown source chain")),
        };

        info!(
            "   Checking fill root sync - Expected: {} | Synced: {}",
            &expected_fill_root[..18],
            &synced_root[..18]
        );

        if synced_root.to_lowercase() == expected_fill_root.to_lowercase() {
            info!("   ✅ Fill root already synced");
            return Ok(());
        }

        info!("   🔄 Fill root out of sync, triggering sync...");

        match (source_chain, dest_chain_id) {
            ("ethereum", MANTLE_CHAIN_ID) => {
                let db_root = self.get_standardized_db_root("mantle_fills")?;
                if db_root != ZERO_LEAF {
                    let root_bytes = self.hex_to_bytes32(&db_root)?;
                    self.ethereum_relayer
                        .sync_dest_chain_fill_root_tx(MANTLE_CHAIN_ID, root_bytes)
                        .await?;
                }
            }
            ("mantle", ETHEREUM_CHAIN_ID) => {
                let db_root = self.get_standardized_db_root("ethereum_fills")?;
                if db_root != ZERO_LEAF {
                    let root_bytes = self.hex_to_bytes32(&db_root)?;
                    self.mantle_relayer
                        .sync_dest_chain_fill_root_tx(ETHEREUM_CHAIN_ID, root_bytes)
                        .await?;
                }
            }
            _ => return Err(anyhow!("Invalid chain combination")),
        }

        let new_synced = match source_chain {
            "ethereum" => self.ethereum_relayer.get_synced_mantle_fill_root().await?,
            "mantle" => self.mantle_relayer.get_synced_ethereum_fill_root().await?,
            _ => unreachable!(),
        };

        info!("   After sync - Synced root: {}", &new_synced[..18]);

        if new_synced.to_lowercase() != expected_fill_root.to_lowercase() {
            return Err(anyhow!(
                "Fill root sync failed. Expected: {}, After sync: {}",
                expected_fill_root,
                new_synced
            ));
        }

        info!("   ✅ Fill root sync completed successfully");
        Ok(())
    }

    async fn get_fill_proof(&self, intent_id: &str, dest_chain: u32) -> Result<(Vec<String>, u32)> {
        match dest_chain {
            ETHEREUM_CHAIN_ID => {
                let proof = self.ethereum_relayer.get_fill_proof(intent_id).await?;
                let index = self.ethereum_relayer.get_fill_index(intent_id).await?;
                Ok((proof, index))
            }
            MANTLE_CHAIN_ID => {
                let proof = self.mantle_relayer.get_fill_proof(intent_id).await?;
                let index = self.mantle_relayer.get_fill_index(intent_id).await?;
                Ok((proof, index))
            }
            _ => Err(anyhow!("Invalid destination chain")),
        }
    }

    fn get_standardized_db_root(&self, tree_name: &str) -> Result<String> {
        let root = self
            .database
            .get_latest_root(tree_name)?
            .unwrap_or_else(|| ZERO_LEAF.to_string());

        Ok(if root.starts_with("0x") {
            root.to_lowercase()
        } else {
            format!("0x{}", root.to_lowercase())
        })
    }

    fn hex_to_bytes32(&self, hex_str: &str) -> Result<[u8; 32]> {
        let s = hex_str.trim_start_matches("0x");
        let vec = hex::decode(s).map_err(|e| anyhow!("Invalid hex format: {}", e))?;
        vec.try_into()
            .map_err(|_| anyhow!("Hex string must be exactly 32 bytes"))
    }

    fn clone_for_task(&self) -> Self {
        Self {
            database: self.database.clone(),
            mantle_relayer: self.mantle_relayer.clone(),
            ethereum_relayer: self.ethereum_relayer.clone(),
            coordinator: self.coordinator.clone(),
            poll_interval: self.poll_interval,
        }
    }
}
