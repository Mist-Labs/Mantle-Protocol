use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use tokio::{
    sync::RwLock,
    time::{self, interval, sleep},
};
use tracing::{error, info};

use crate::{
    database::database::Database,
    encryption::encryption_utils::decrypt_with_ecies,
    merkle_manager::merkle_manager::MerkleTreeManager,
    models::{
        model::{BridgeMetrics, Intent, IntentOperationState, IntentStatus, TokenType},
        traits::ChainRelayer,
    },
    relay_coordinator::model::{BridgeCoordinator, EthereumRelayer, MantleRelayer},
};

impl TokenType {
    pub fn from_address(address: &str) -> Result<Self> {
        match address.to_lowercase().as_str() {
            "0x0000000000000000000000000000000000000000" => Ok(Self::ETH),
            "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" => Ok(Self::ETH),
            "0x28650373758d75a8ff0b22587f111e47bac34e21" => Ok(Self::USDC),
            "0x89f4f0e13997ca27ceb963dee291c607e4e59923" => Ok(Self::USDT),
            "0x50e8da97beeb8064714de45ce1f250879f3bd5b5" => Ok(Self::WETH),
            "0x65e37b558f64e2be5768db46df22f93d85741a9e" => Ok(Self::MNT),
            "0xa4b184006b59861f80521649b14e4e8a72499a23" => Ok(Self::USDC),
            "0xb0ee6ef7788e9122fc4aae327ed4fef56c7da891" => Ok(Self::USDT),
            "0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111" => Ok(Self::WETH),
            "0x44fce297e4d6c5a50d28fb26a58202e4d49a13e7" => Ok(Self::MNT),
            _ => Err(anyhow!("Unsupported token address: {}", address)),
        }
    }

    pub fn from_symbol(symbol: &str) -> Result<Self> {
        match symbol.to_uppercase().as_str() {
            "ETH" => Ok(Self::ETH),
            "USDC" => Ok(Self::USDC),
            "USDT" => Ok(Self::USDT),
            "WETH" => Ok(Self::WETH),
            "MNT" => Ok(Self::MNT),
            _ => Err(anyhow!("Unsupported token symbol: {}", symbol)),
        }
    }

    pub fn get_ethereum_address(&self) -> &str {
        match self {
            Self::ETH => "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            Self::USDC => "0x28650373758d75a8fF0B22587F111e47BAC34e21",
            Self::USDT => "0x89F4f0e13997Ca27cEB963DEE291C607e4E59923",
            Self::WETH => "0x50e8Da97BeEB8064714dE45ce1F250879f3bD5B5",
            Self::MNT => "0x65e37B558F64E2Be5768DB46DF22F93d85741A9E",
        }
    }

    pub fn get_mantle_address(&self) -> &str {
        match self {
            Self::ETH => "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            Self::USDC => "0xA4b184006B59861f80521649b14E4E8A72499A23",
            Self::USDT => "0xB0ee6EF7788E9122fc4AAE327Ed4FEf56c7da891",
            Self::WETH => "0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111",
            Self::MNT => "0x44FCE297e4D6c5A50D28Fb26A58202e4D49a13E7",
        }
    }

    pub fn get_decimals(&self) -> u8 {
        match self {
            Self::ETH | Self::WETH | Self::MNT => 18,
            Self::USDC | Self::USDT => 6,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Self::ETH => "ETH",
            Self::USDC => "USDC",
            Self::USDT => "USDT",
            Self::WETH => "WETH",
            Self::MNT => "MNT",
        }
    }
}

impl Default for BridgeMetrics {
    fn default() -> Self {
        Self {
            total_intents_processed: 0,
            successful_bridges: 0,
            failed_intents: 0,
            refunded_intents: 0,
            ethereum_fills: 0,
            mantle_fills: 0,
            ethereum_claims: 0,
            mantle_claims: 0,
            retry_attempts: 0,
            last_error: None,
            uptime_seconds: 0,
            volumes_by_token: HashMap::new(),
        }
    }
}

impl BridgeMetrics {
    pub fn to_json(&self) -> serde_json::Value {
        let volumes: HashMap<String, String> = self
            .volumes_by_token
            .iter()
            .map(|(k, v)| (k.symbol().to_string(), v.to_string()))
            .collect();

        serde_json::json!({
            "total_intents_processed": self.total_intents_processed,
            "successful_bridges": self.successful_bridges,
            "failed_intents": self.failed_intents,
            "refunded_intents": self.refunded_intents,
            "ethereum_fills": self.ethereum_fills,
            "mantle_fills": self.mantle_fills,
            "ethereum_claims": self.ethereum_claims,
            "mantle_claims": self.mantle_claims,
            "retry_attempts": self.retry_attempts,
            "last_error": self.last_error,
            "uptime_seconds": self.uptime_seconds,
            "volumes_by_token": volumes,
        })
    }
}

impl BridgeCoordinator {
    pub fn new(
        ethereum_relayer: Arc<EthereumRelayer>,
        mantle_relayer: Arc<MantleRelayer>,
        database: Arc<Database>,
        merkle_tree_manager: Arc<MerkleTreeManager>,
    ) -> Self {
        Self {
            ethereum_relayer,
            mantle_relayer,
            database,
            merkle_tree_manager,
            metrics: Arc::new(RwLock::new(BridgeMetrics::default())),
            operation_states: Arc::new(RwLock::new(HashMap::new())),
            start_time: time::Instant::now(),
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        info!("🌉 Bridge coordinator started (Across-style SpokePool)");

        let metrics = Arc::clone(&self.metrics);
        let start_time = self.start_time;
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let mut m = metrics.write().await;
                m.uptime_seconds = start_time.elapsed().as_secs();
            }
        });

        let merkle_manager = Arc::clone(&self.merkle_tree_manager);
        tokio::spawn(async move {
            if let Err(e) = merkle_manager.start().await {
                error!("❌ Merkle manager failed: {}", e);
            }
        });

        loop {
            if let Err(e) = self.process_pending_intents().await {
                error!("❌ Error processing intents: {}", e);
                self.record_error(e.to_string()).await;
            }
            sleep(Duration::from_secs(10)).await;
        }
    }

    async fn process_pending_intents(&self) -> Result<()> {
        let pending_intents = self
            .database
            .get_pending_intents()
            .map_err(|e| anyhow!("Failed to get pending intents: {}", e))?;

        if pending_intents.is_empty() {
            return Ok(());
        }

        for intent in pending_intents {
            {
                let mut metrics = self.metrics.write().await;
                metrics.total_intents_processed += 1;
            }

            if intent.status == IntentStatus::SolverPaid {
                if let Err(e) = self.claim_for_user(&intent).await {
                    error!("Failed to claim for user (intent {}): {}", intent.id, e);
                    self.record_error(format!("Claim failed: {}", e)).await;
                }
            }
        }

        Ok(())
    }

    pub async fn claim_for_user(&self, intent: &Intent) -> Result<()> {
        match intent.status {
            IntentStatus::SolverPaid => {
                info!("💸 Claiming for user on {}", intent.dest_chain);

                match intent.dest_chain.as_str() {
                    "mantle" | "5003" => {
                        self.claim_on_chain(&*self.mantle_relayer, intent, true)
                            .await
                    }
                    "ethereum" | "11155111" => {
                        self.claim_on_chain(&*self.ethereum_relayer, intent, false)
                            .await
                    }
                    _ => Err(anyhow!(
                        "Unsupported destination chain: {}",
                        intent.dest_chain
                    )),
                }
            }
            IntentStatus::Registered | IntentStatus::Filled => {
                let now = chrono::Utc::now().timestamp() as u64;
                if now > intent.deadline {
                    info!("⏰ Intent {} expired, refunding", intent.id);
                    self.handle_refund(intent).await
                } else {
                    Ok(())
                }
            }
            IntentStatus::UserClaimed | IntentStatus::Refunded => Ok(()),
            _ => Ok(()),
        }
    }

    async fn claim_on_chain<T: ChainRelayer>(
        &self,
        relayer: &T,
        intent: &Intent,
        is_mantle: bool,
    ) -> Result<()> {
        info!(
            "🔓 Claiming on {} for intent {}",
            if is_mantle { "Mantle" } else { "Ethereum" },
            intent.id
        );

        let privacy_params = self
            .database
            .get_intent_privacy_params(&intent.id)
            .map_err(|e| anyhow!("Failed to get privacy params: {}", e))?;

        let encrypted_secret = privacy_params
            .secret
            .as_ref()
            .ok_or_else(|| anyhow!("Encrypted secret not available"))?;

        let encrypted_nullifier = privacy_params
            .nullifier
            .as_ref()
            .ok_or_else(|| anyhow!("Encrypted nullifier not available"))?;

        let recipient = privacy_params
            .recipient
            .as_ref()
            .ok_or_else(|| anyhow!("Recipient not available"))?;

        let claim_auth_hex = privacy_params
            .claim_signature
            .as_ref()
            .ok_or_else(|| anyhow!("Claim signature not available"))?;

        let relayer_private_key = std::env::var("RELAYER_PRIVATE_KEY")
            .map_err(|_| anyhow!("RELAYER_PRIVATE_KEY not set"))?;

        let secret = decrypt_with_ecies(encrypted_secret, &relayer_private_key)
            .map_err(|e| anyhow!("Failed to decrypt secret: {}", e))?;

        let nullifier = decrypt_with_ecies(encrypted_nullifier, &relayer_private_key)
            .map_err(|e| anyhow!("Failed to decrypt nullifier: {}", e))?;

        let claim_auth_hex_clean = claim_auth_hex.strip_prefix("0x").unwrap_or(claim_auth_hex);
        let claim_auth_bytes = hex::decode(claim_auth_hex_clean)
            .map_err(|e| anyhow!("Failed to decode claim signature hex: {}", e))?;

        if claim_auth_bytes.len() != 65 {
            return Err(anyhow!(
                "Invalid signature length: expected 65 bytes, got {}",
                claim_auth_bytes.len()
            ));
        }

        let result = relayer
            .claim_withdrawal(
                &intent.id,
                &nullifier,
                recipient,
                &secret,
                &claim_auth_bytes,
            )
            .await;

        match result {
            Ok(txid) => {
                info!(
                    "✅ Claimed on {}: {}",
                    if is_mantle { "Mantle" } else { "Ethereum" },
                    txid
                );

                self.database
                    .update_intent_status(&intent.id, IntentStatus::UserClaimed)
                    .map_err(|e| anyhow!("Failed to update status: {}", e))?;

                let mut metrics = self.metrics.write().await;
                if is_mantle {
                    metrics.mantle_claims += 1;
                } else {
                    metrics.ethereum_claims += 1;
                }
                Ok(())
            }
            Err(e) => {
                error!("❌ Claim failed: {}", e);
                Err(anyhow!("Claim failed: {}", e))
            }
        }
    }

    pub async fn handle_refund(&self, intent: &Intent) -> Result<()> {
        info!(
            "♻️ Refunding intent {} on {}",
            intent.id, intent.source_chain
        );

        let result = match intent.source_chain.as_str() {
            "ethereum" | "11155111" => self.ethereum_relayer.refund_intent(&intent.id).await,
            "mantle" | "5003" => self.mantle_relayer.refund_intent(&intent.id).await,
            _ => return Err(anyhow!("Unsupported source chain: {}", intent.source_chain)),
        };

        match result {
            Ok(_) => {
                info!("✅ Intent {} refunded on-chain", intent.id);
            }
            Err(e) if e.to_string().contains("0x53857a9d") => {
                info!("ℹ️ Intent {} already processed, syncing state", intent.id);
            }
            Err(e) => {
                return Err(anyhow!("Refund failed: {}", e));
            }
        }

        self.database
            .update_intent_status(&intent.id, IntentStatus::Refunded)
            .map_err(|e| anyhow!("Failed to update status: {}", e))?;

        let mut metrics = self.metrics.write().await;
        metrics.refunded_intents += 1;

        info!("♻️ Intent {} marked as Refunded", intent.id);
        Ok(())
    }

    async fn record_error(&self, error: String) {
        let mut metrics = self.metrics.write().await;
        metrics.last_error = Some(error);
    }

    pub async fn get_metrics(&self) -> BridgeMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn get_operation_states(&self) -> Vec<IntentOperationState> {
        self.operation_states
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    pub fn is_token_supported(&self, token_address: &str, chain_id: u32) -> bool {
        TokenType::from_address(token_address)
            .map(|token_type| {
                let dest_address = match chain_id {
                    11155111 => token_type.get_ethereum_address(),
                    5003 => token_type.get_mantle_address(),
                    _ => return false,
                };
                dest_address != "0x0000000000000000000000000000000000000000"
                    || token_type == TokenType::ETH
            })
            .unwrap_or(false)
    }
}
