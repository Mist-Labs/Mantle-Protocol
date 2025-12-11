use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use tokio::{sync::RwLock, time::interval};
use tracing::{debug, error, info, warn};

use crate::{
    database::database::Database,
    models::model::TokenType,
    relay_coordinator::model::{EthereumRelayer, MantleRelayer, SecretMonitor, SecretMonitorStats},
};

impl SecretMonitorStats {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "processed_nullifiers": self.processed_nullifiers,
            "ethereum_check_interval_secs": self.ethereum_check_interval_secs,
            "mantle_check_interval_secs": self.mantle_check_interval_secs,
        })
    }
}

impl SecretMonitor {
    pub fn new(
        ethereum_relayer: Arc<EthereumRelayer>,
        mantle_relayer: Arc<MantleRelayer>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            ethereum_relayer,
            mantle_relayer,
            database,
            processed_nullifiers: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("🔍 Starting bridge secret monitor");

        tokio::try_join!(
            self.monitor_ethereum_secrets(),
            self.monitor_mantle_secrets()
        )?;

        Ok(())
    }

    async fn monitor_ethereum_secrets(&self) -> Result<()> {
        let mut check_interval = interval(Duration::from_secs(12)); // Ethereum block time

        loop {
            check_interval.tick().await;

            match self.check_ethereum_claims().await {
                Ok(_) => {}
                Err(e) => {
                    error!("❌ Error monitoring Ethereum secrets: {}", e);
                }
            }
        }
    }

    async fn check_ethereum_claims(&self) -> Result<()> {
        let pending_intents = self
            .database
            .get_intents_awaiting_secret()
            .map_err(|e| anyhow!("Failed to get pending intents: {}", e))?;

        if pending_intents.is_empty() {
            debug!("No intents awaiting secrets on Ethereum");
            return Ok(());
        }

        for intent in pending_intents {
            let privacy_params = self
                .database
                .get_intent_privacy_params(&intent.id)
                .map_err(|e| anyhow!("Failed to get privacy params: {}", e))?;

            if privacy_params.secret.is_some() {
                continue;
            }

            if intent.dest_chain != "Ethereum" {
                continue;
            }

            let nullifier = match &privacy_params.nullifier {
                Some(n) => n,
                None => continue,
            };

            {
                let processed = self.processed_nullifiers.read().await;
                if processed.contains(nullifier) {
                    continue;
                }
            }

            match self
                .check_ethereum_withdrawal_event(&intent.id, nullifier)
                .await
            {
                Ok(Some((secret, token_address))) => {
                    let token_type = TokenType::from_address(&token_address).ok();
                    let token_symbol = token_type.as_ref().map(|t| t.symbol()).unwrap_or("UNKNOWN");

                    info!(
                        "🔑 Discovered Ethereum secret for intent: {} token: {}",
                        intent.id, token_symbol
                    );

                    self.database
                        .update_intent_secret(&intent.id, &secret)
                        .map_err(|e| anyhow!("Failed to update secret: {}", e))?;

                    let mut processed = self.processed_nullifiers.write().await;
                    processed.insert(nullifier.clone());

                    info!("✅ Secret saved for {} intent {}", token_symbol, intent.id);
                }
                Ok(None) => {
                    debug!("⏳ No secret yet for nullifier {} on Ethereum", nullifier);
                }
                Err(e) => {
                    warn!(
                        "⚠️ Error checking Ethereum nullifier {} (will retry): {}",
                        nullifier, e
                    );
                }
            }
        }

        Ok(())
    }

    async fn check_ethereum_withdrawal_event(
        &self,
        intent_id: &str,
        nullifier: &str,
    ) -> Result<Option<(String, String)>> {
        let event =
            match self
                .database
                .get_bridge_event_by_nullifier(nullifier, "WithdrawalClaimed", 1)
            {
                Ok(Some(evt)) => evt,
                Ok(None) => {
                    debug!(
                        "No WithdrawalClaimed event found yet for nullifier {}",
                        nullifier
                    );
                    return Ok(None);
                }
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to query indexer for nullifier {}: {}",
                        nullifier,
                        e
                    ));
                }
            };

        let secret = event
            .get("secret")
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow!("Secret not found in event data"))?;

        let token_address = event
            .get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("Token address not found in event data"))?;

        let event_nullifier = event
            .get("nullifier")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("Nullifier not found in event data"))?;

        if event_nullifier != nullifier {
            return Err(anyhow!("Nullifier mismatch in event data"));
        }

        info!("🔍 Found secret in Ethereum WithdrawalClaimed event");
        Ok(Some((secret.to_string(), token_address.to_string())))
    }

    async fn monitor_mantle_secrets(&self) -> Result<()> {
        let mut check_interval = interval(Duration::from_secs(2)); // Mantle ~2s block time

        loop {
            check_interval.tick().await;

            match self.check_mantle_claims().await {
                Ok(_) => {}
                Err(e) => {
                    error!("❌ Error monitoring Mantle secrets: {}", e);
                }
            }
        }
    }

    async fn check_mantle_claims(&self) -> Result<()> {
        let pending_intents = self
            .database
            .get_intents_awaiting_secret()
            .map_err(|e| anyhow!("Failed to get pending intents: {}", e))?;

        if pending_intents.is_empty() {
            debug!("No intents awaiting secrets on Mantle");
            return Ok(());
        }

        for intent in pending_intents {
            let privacy_params = self
                .database
                .get_intent_privacy_params(&intent.id)
                .map_err(|e| anyhow!("Failed to get privacy params: {}", e))?;

            if privacy_params.secret.is_some() {
                continue;
            }

            if intent.dest_chain != "mantle" {
                continue;
            }

            let nullifier = match &privacy_params.nullifier {
                Some(n) => n,
                None => continue,
            };

            {
                let processed = self.processed_nullifiers.read().await;
                if processed.contains(nullifier) {
                    continue;
                }
            }

            match self
                .check_mantle_withdrawal_event(&intent.id, nullifier)
                .await
            {
                Ok(Some((secret, token_address))) => {
                    let token_type = TokenType::from_address(&token_address).ok();
                    let token_symbol = token_type.as_ref().map(|t| t.symbol()).unwrap_or("UNKNOWN");

                    info!(
                        "🔑 Discovered Mantle secret for intent {} ({}): {}",
                        intent.id, token_symbol, secret
                    );

                    self.database
                        .update_intent_secret(&intent.id, &secret)
                        .map_err(|e| anyhow!("Failed to update secret: {}", e))?;

                    let mut processed = self.processed_nullifiers.write().await;
                    processed.insert(nullifier.clone().to_string());

                    info!("✅ Secret saved for {} intent {}", token_symbol, intent.id);
                }
                Ok(None) => {
                    debug!("⏳ No secret yet for nullifier {} on Mantle", nullifier);
                }
                Err(e) => {
                    warn!(
                        "⚠️ Error checking Mantle nullifier {} (will retry): {}",
                        nullifier, e
                    );
                }
            }
        }

        Ok(())
    }

    async fn check_mantle_withdrawal_event(
        &self,
        intent_id: &str,
        nullifier: &str,
    ) -> Result<Option<(String, String)>> {
        // Query indexer for WithdrawalClaimed event
        let event =
            match self
                .database
                .get_bridge_event_by_nullifier(nullifier, "WithdrawalClaimed", 5000)
            {
                Ok(Some(evt)) => evt,
                Ok(None) => {
                    debug!(
                        "No WithdrawalClaimed event found yet for nullifier {}",
                        nullifier
                    );
                    return Ok(None);
                }
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to query indexer for nullifier {}: {}",
                        nullifier,
                        e
                    ));
                }
            };

        let secret = event
            .get("secret")
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow!("Secret not found in event data"))?;

        let token_address = event
            .get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("Token address not found in event data"))?;

        let event_nullifier = event
            .get("nullifier")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("Nullifier not found in event data"))?;

        if event_nullifier != nullifier {
            return Err(anyhow!("Nullifier mismatch in event data"));
        }

        info!(
            "🔍 Found secret in Mantle WithdrawalClaimed event: {}",
            secret
        );
        Ok(Some((secret.to_string(), token_address.to_string())))
    }

    pub async fn get_stats(&self) -> SecretMonitorStats {
        let processed_count = self.processed_nullifiers.read().await.len();

        SecretMonitorStats {
            processed_nullifiers: processed_count,
            ethereum_check_interval_secs: 12,
            mantle_check_interval_secs: 2,
        }
    }
}
