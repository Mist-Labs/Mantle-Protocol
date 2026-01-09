use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

use crate::{
    database::database::Database,
    models::model::{Intent, IntentStatus},
    relay_coordinator::model::{EthereumRelayer, MantleRelayer},
};

const ETHEREUM_CHAIN_ID: u32 = 11155111;
const MANTLE_CHAIN_ID: u32 = 5003;
const MAX_CONCURRENT_SETTLEMENTS: usize = 3;
const ZERO_LEAF: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

pub struct IntentSettlementWorker {
    database: Arc<Database>,
    mantle_relayer: Arc<MantleRelayer>,
    ethereum_relayer: Arc<EthereumRelayer>,
    poll_interval: Duration,
}

impl IntentSettlementWorker {
    pub fn new(
        database: Arc<Database>,
        mantle_relayer: Arc<MantleRelayer>,
        ethereum_relayer: Arc<EthereumRelayer>,
    ) -> Self {
        Self {
            database,
            mantle_relayer,
            ethereum_relayer,
            poll_interval: Duration::from_secs(15),
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
        let (source_chain, dest_chain_id) = match intent.source_chain.as_str() {
            "ethereum" => ("ethereum", MANTLE_CHAIN_ID),
            "mantle" => ("mantle", ETHEREUM_CHAIN_ID),
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

        let (local_fill_root, onchain_synced_root) = match (source_chain, dest_chain_id) {
            ("ethereum", MANTLE_CHAIN_ID) => {
                let local = self.get_standardized_db_root("mantle_fills")?;
                let onchain = self.ethereum_relayer.get_synced_mantle_fill_root().await?;
                (local, onchain.to_lowercase())
            }
            ("mantle", ETHEREUM_CHAIN_ID) => {
                let local = self.get_standardized_db_root("ethereum_fills")?;
                let onchain = self.mantle_relayer.get_synced_ethereum_fill_root().await?;
                (local, onchain.to_lowercase())
            }
            _ => return Err(anyhow!("Invalid chain combination")),
        };

        if local_fill_root != onchain_synced_root {
            warn!(
                "🚧 Sync gap: DB {} != On-chain {}",
                &local_fill_root[..10],
                &onchain_synced_root[..10]
            );
            return Ok(());
        }

        info!(
            "✅ Root synced ({}), generating proof",
            &local_fill_root[..10]
        );
        let (fill_proof, leaf_index) = self.get_fill_proof(&intent.id, dest_chain_id).await?;

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

        info!("✅ Intent {} settled: {}", &intent.id[..10], tx_hash);
        Ok(())
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

    fn clone_for_task(&self) -> Self {
        Self {
            database: self.database.clone(),
            mantle_relayer: self.mantle_relayer.clone(),
            ethereum_relayer: self.ethereum_relayer.clone(),
            poll_interval: self.poll_interval,
        }
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
}
