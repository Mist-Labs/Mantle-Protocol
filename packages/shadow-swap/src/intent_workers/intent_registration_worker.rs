use anyhow::{Context, Result, anyhow};
use ethers::types::U256;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

use crate::{
    database::database::Database,
    merkle_manager::merkle_manager::MerkleTreeManager,
    models::model::{Intent, IntentStatus, TokenType},
    relay_coordinator::model::{EthereumRelayer, MantleRelayer},
    root_sync_coordinator::root_sync_coordinator::RootSyncCoordinator,
};

const ETHEREUM_CHAIN_ID: u32 = 11155111;
const MANTLE_CHAIN_ID: u32 = 5003;
const MAX_CONCURRENT_REGISTRATIONS: usize = 5;

pub struct IntentRegistrationWorker {
    database: Arc<Database>,
    mantle_relayer: Arc<MantleRelayer>,
    ethereum_relayer: Arc<EthereumRelayer>,
    merkle_manager: Arc<MerkleTreeManager>,
    root_sync_coordinator: Arc<RootSyncCoordinator>,
    poll_interval: Duration,
}

impl IntentRegistrationWorker {
    pub fn new(
        database: Arc<Database>,
        mantle_relayer: Arc<MantleRelayer>,
        ethereum_relayer: Arc<EthereumRelayer>,
        merkle_manager: Arc<MerkleTreeManager>,
        root_sync_coordinator: Arc<RootSyncCoordinator>,
    ) -> Self {
        Self {
            database,
            mantle_relayer,
            ethereum_relayer,
            merkle_manager,
            root_sync_coordinator,
            poll_interval: Duration::from_secs(10),
        }
    }

    pub async fn run(&self) {
        info!("🔄 Intent registration worker started");

        loop {
            if let Err(e) = self.process_pending_registrations().await {
                error!("Registration worker error: {}", e);
            }
            sleep(self.poll_interval).await;
        }
    }

    async fn process_pending_registrations(&self) -> Result<()> {
        let pending = self
            .database
            .get_intents_by_status(IntentStatus::Committed)
            .context("Failed to fetch pending intents")?;

        if pending.is_empty() {
            return Ok(());
        }

        info!("📋 Found {} intents pending registration", pending.len());

        let mut tasks = Vec::new();

        for intent in pending.into_iter().take(MAX_CONCURRENT_REGISTRATIONS) {
            let worker = self.clone_for_task();
            let task = tokio::spawn(async move {
                let intent_id = intent.id.clone();
                match worker.process_single_intent_with_retry(&intent).await {
                    Ok(_) => info!("✅ Processed intent {}", &intent_id[..10]),
                    Err(e) => error!("❌ Failed to process intent {}: {:#?}", &intent_id[..10], e),
                }
            });
            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    fn clone_for_task(&self) -> Self {
        Self {
            database: self.database.clone(),
            mantle_relayer: self.mantle_relayer.clone(),
            ethereum_relayer: self.ethereum_relayer.clone(),
            merkle_manager: self.merkle_manager.clone(),
            root_sync_coordinator: self.root_sync_coordinator.clone(),
            poll_interval: self.poll_interval,
        }
    }

    async fn process_single_intent_with_retry(&self, intent: &Intent) -> Result<()> {
        let mut last_error = None;

        for attempt in 1..=3 {
            match self.process_single_intent(intent).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    warn!(
                        "Attempt {}/3 failed for intent {}: {}",
                        attempt,
                        &intent.id[..10],
                        e
                    );
                    last_error = Some(e);

                    if attempt < 3 {
                        sleep(Duration::from_secs(2 * attempt as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    async fn process_single_intent(&self, intent: &Intent) -> Result<()> {
        if intent.deadline < chrono::Utc::now().timestamp() as u64 {
            warn!("Intent {} expired, processing refund", &intent.id[..10]);
            self.database
                .update_intent_status(&intent.id, IntentStatus::Expired)?;

            match intent.source_chain.as_str() {
                "mantle" => match self.mantle_relayer.execute_refund(&intent.id).await {
                    Ok(tx_hash) => info!("✅ Refunded on Mantle: {}", tx_hash),
                    Err(e) => error!("❌ Mantle refund failed: {:#?}", e),
                },
                "ethereum" => match self.ethereum_relayer.execute_refund(&intent.id).await {
                    Ok(tx_hash) => info!("✅ Refunded on Ethereum: {}", tx_hash),
                    Err(e) => error!("❌ Ethereum refund failed: {:#?}", e),
                },
                _ => warn!("Unknown source chain for refund"),
            }
            return Ok(());
        }

        let commitment = intent
            .source_commitment
            .as_ref()
            .ok_or_else(|| anyhow!("Missing commitment"))?;

        match intent.source_chain.as_str() {
            "ethereum" => {
                if self.check_already_registered_on_mantle(&intent.id).await? {
                    info!(
                        "✅ Intent {} already registered on Mantle",
                        &intent.id[..10]
                    );
                    self.database
                        .update_intent_status(&intent.id, IntentStatus::Registered)?;
                    return Ok(());
                }
                self.register_on_mantle(intent, commitment).await
            }
            "mantle" => {
                if self
                    .check_already_registered_on_ethereum(&intent.id)
                    .await?
                {
                    info!(
                        "✅ Intent {} already registered on Ethereum",
                        &intent.id[..10]
                    );
                    self.database
                        .update_intent_status(&intent.id, IntentStatus::Registered)?;
                    return Ok(());
                }
                self.register_on_ethereum(intent, commitment).await
            }
            chain => Err(anyhow!("Unsupported source chain: {}", chain)),
        }
    }

    async fn check_already_registered_on_mantle(&self, intent_id: &str) -> Result<bool> {
        self.mantle_relayer.check_intent_registered(intent_id).await
    }

    async fn check_already_registered_on_ethereum(&self, intent_id: &str) -> Result<bool> {
        self.ethereum_relayer
            .check_intent_registered(intent_id)
            .await
    }

    async fn register_on_ethereum(&self, intent: &Intent, commitment: &str) -> Result<()> {
        info!("📝 [Ethereum] Registering intent {}", &intent.id[..10]);

        info!("   Rebuilding Mantle commitments tree...");
        self.merkle_manager
            .rebuild_mantle_commitments_tree()
            .await?;

        let db_root = self
            .database
            .get_latest_root("mantle_commitments")?
            .ok_or_else(|| anyhow!("Mantle commitments root not found"))?;

        info!("   DB root (Mantle): {}", &db_root[..18]);

        let sync_result = tokio::time::timeout(
            Duration::from_secs(120),
            self.ensure_root_synced_on_ethereum(&db_root),
        )
        .await;

        match sync_result {
            Ok(Ok(())) => info!("   ✅ Root synced to Ethereum"),
            Ok(Err(e)) => return Err(anyhow!("Root sync failed: {}", e)),
            Err(_) => return Err(anyhow!("Root sync timeout after 2min")),
        }

        let tree_meta = self
            .database
            .get_merkle_tree_by_name("mantle_commitments")?
            .ok_or_else(|| anyhow!("Mantle tree metadata not found"))?;

        let proof_gen = self.merkle_manager.get_proof_generator();
        let (proof, commitment_index, root) =
            proof_gen.generate_proof("mantle", commitment, tree_meta.leaf_count as usize)?;

        info!(
            "   Proof generated - Index: {}, Length: {}",
            commitment_index,
            proof.len()
        );

        let token_type = TokenType::from_address(&intent.source_token)?;
        let dest_token = token_type.get_ethereum_address();
        let dest_amount =
            self.convert_amount(&intent.dest_amount, &intent.source_token, dest_token)?;

        let txid = self
            .ethereum_relayer
            .register_intent(
                &intent.id,
                commitment,
                dest_token,
                &dest_amount,
                MANTLE_CHAIN_ID,
                intent.deadline,
                &root,
                &proof,
                commitment_index as u32,
            )
            .await?;

        self.database
            .update_dest_registration_txid(&intent.id, &txid)?;
        self.database
            .update_intent_status(&intent.id, IntentStatus::Registered)?;

        info!("🎉 [Ethereum] Successfully registered: {}", txid);
        Ok(())
    }

    async fn register_on_mantle(&self, intent: &Intent, commitment: &str) -> Result<()> {
        info!("📝 [Mantle] Registering intent {}", &intent.id[..10]);

        info!("   Rebuilding Ethereum commitments tree...");
        self.merkle_manager
            .rebuild_ethereum_commitments_tree()
            .await?;

        let db_root = self
            .database
            .get_latest_root("ethereum_commitments")?
            .ok_or_else(|| anyhow!("Ethereum commitments root not found"))?;

        info!("   DB root (Ethereum): {}", &db_root[..18]);

        let sync_result = tokio::time::timeout(
            Duration::from_secs(120),
            self.ensure_root_synced_on_mantle(&db_root),
        )
        .await;

        match sync_result {
            Ok(Ok(())) => info!("   ✅ Root synced to Mantle"),
            Ok(Err(e)) => return Err(anyhow!("Root sync failed: {}", e)),
            Err(_) => return Err(anyhow!("Root sync timeout after 2min")),
        }

        let tree_meta = self
            .database
            .get_merkle_tree_by_name("ethereum_commitments")?
            .ok_or_else(|| anyhow!("Ethereum tree metadata not found"))?;

        let proof_gen = self.merkle_manager.get_proof_generator();
        let (proof, commitment_index, root) =
            proof_gen.generate_proof("ethereum", commitment, tree_meta.leaf_count as usize)?;

        info!(
            "   Proof generated - Index: {}, Length: {}",
            commitment_index,
            proof.len()
        );

        let token_type = TokenType::from_address(&intent.source_token)?;
        let dest_token = token_type.get_mantle_address();
        let dest_amount =
            self.convert_amount(&intent.dest_amount, &intent.source_token, dest_token)?;

        let txid = self
            .mantle_relayer
            .register_intent(
                &intent.id,
                commitment,
                dest_token,
                &dest_amount,
                ETHEREUM_CHAIN_ID,
                intent.deadline,
                &root,
                &proof,
                commitment_index as u32,
            )
            .await?;

        self.database
            .update_dest_registration_txid(&intent.id, &txid)?;
        self.database
            .update_intent_status(&intent.id, IntentStatus::Registered)?;

        info!("🎉 [Mantle] Successfully registered: {}", txid);
        Ok(())
    }

    async fn ensure_root_synced_on_ethereum(&self, expected_root: &str) -> Result<()> {
        let synced = self
            .ethereum_relayer
            .get_synced_mantle_commitment_root()
            .await?;

        info!("   Ethereum's view of Mantle root: {}", &synced[..18]);

        if synced.to_lowercase() == expected_root.to_lowercase() {
            info!("   ✅ Already synced");
            return Ok(());
        }

        info!("   🔄 Syncing root to Ethereum...");
        self.root_sync_coordinator
            .sync_mantle_commitments_to_ethereum()
            .await?;

        let new_synced = self
            .ethereum_relayer
            .get_synced_mantle_commitment_root()
            .await?;

        if new_synced.to_lowercase() != expected_root.to_lowercase() {
            return Err(anyhow!(
                "Root sync failed. Expected: {}, Got: {}",
                expected_root,
                new_synced
            ));
        }

        info!("   ✅ Root synced successfully");
        Ok(())
    }

    async fn ensure_root_synced_on_mantle(&self, expected_root: &str) -> Result<()> {
        let synced = self
            .mantle_relayer
            .get_synced_ethereum_commitment_root()
            .await?;

        info!("   Mantle's view of Ethereum root: {}", &synced[..18]);

        if synced.to_lowercase() == expected_root.to_lowercase() {
            info!("   ✅ Already synced");
            return Ok(());
        }

        info!("   🔄 Syncing root to Mantle...");
        self.root_sync_coordinator
            .sync_ethereum_commitments_to_mantle()
            .await?;

        let new_synced = self
            .mantle_relayer
            .get_synced_ethereum_commitment_root()
            .await?;

        if new_synced.to_lowercase() != expected_root.to_lowercase() {
            return Err(anyhow!(
                "Root sync failed. Expected: {}, Got: {}",
                expected_root,
                new_synced
            ));
        }

        info!("   ✅ Root synced successfully");
        Ok(())
    }

    fn convert_amount(&self, amount: &str, source_token: &str, dest_token: &str) -> Result<String> {
        let source_type = TokenType::from_address(source_token)?;
        let dest_type = TokenType::from_address(dest_token)?;

        let source_decimals = source_type.get_decimals();
        let dest_decimals = dest_type.get_decimals();

        let amount_u256 = U256::from_dec_str(amount).context("Invalid amount format")?;

        let converted = if dest_decimals > source_decimals {
            let diff = dest_decimals - source_decimals;
            let multiplier = U256::from(10u64).pow(U256::from(diff));
            amount_u256
                .checked_mul(multiplier)
                .ok_or_else(|| anyhow!("Amount overflow"))?
        } else if source_decimals > dest_decimals {
            let diff = source_decimals - dest_decimals;
            let divisor = U256::from(10u64).pow(U256::from(diff));
            amount_u256
                .checked_div(divisor)
                .ok_or_else(|| anyhow!("Amount underflow"))?
        } else {
            amount_u256
        };

        Ok(converted.to_string())
    }
}
