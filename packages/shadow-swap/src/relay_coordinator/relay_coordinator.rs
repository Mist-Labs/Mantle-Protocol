use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use tokio::{
    sync::RwLock,
    time::{self, interval, sleep},
};
use tracing::{debug, error, info, warn};

use crate::{
    database::database::Database,
    merkle_manager::merkletreemanager::MerkleTreeManager,
    models::{
        model::{
            BridgeDirection, BridgeMetrics, Intent, IntentOperationState, IntentStatus,
            TokenBridgeInfo, TokenType,
        },
        traits::ChainRelayer,
    },
    relay_coordinator::model::{BridgeCoordinator, EthereumRelayer, MantleRelayer},
};

impl TokenType {
    pub fn from_address(address: &str) -> Result<Self> {
        match address.to_lowercase().as_str() {
            "0x0000000000000000000000000000000000000000" => Ok(Self::ETH),
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => Ok(Self::USDC),
            "0xdac17f958d2ee523a2206206994597c13d831ec7" => Ok(Self::USDT),
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => Ok(Self::WETH),
            "0x3c3a81e81dc49A522A592e7622A7E711c06bf354" => Ok(Self::MNT),

            "0x6b175474e89094c44da98b954eedeac495271d0f" => Ok(Self::DAI),
            "0x09bc4e0d864854c6afb6eb9a9cdfe58c4fcaa6e5" => Ok(Self::USDC),
            "0x201eba5cc46d216ce6dc03f6a759e8e766e956ae" => Ok(Self::USDT),
            "0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111" => Ok(Self::WETH),
            "0xdeaddeaddeaddeaddeaddeaddeaddeaddead0000" => Ok(Self::MNT),
            _ => Err(anyhow!("Unsupported token address: {}", address)),
        }
    }

    pub fn from_symbol(symbol: &str) -> Result<Self> {
        match symbol.to_uppercase().as_str() {
            "ETH" => Ok(Self::ETH),
            "USDC" => Ok(Self::USDC),
            "USDT" => Ok(Self::USDT),
            "WETH" => Ok(Self::WETH),
            "DAI" => Ok(Self::DAI),
            "MNT" => Ok(Self::MNT),
            _ => Err(anyhow::anyhow!("Unsupported token symbol: {}", symbol)),
        }
    }

    pub fn get_ethereum_address(&self) -> &str {
        match self {
            Self::ETH => "0x0000000000000000000000000000000000000000",
            Self::USDC => "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
            Self::USDT => "0xdac17f958d2ee523a2206206994597c13d831ec7",
            Self::WETH => "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            Self::DAI => "0x6b175474e89094c44da98b954eedeac495271d0f",
            Self::MNT => "0x3c3a81e81dc49A522A592e7622A7E711c06bf354",
        }
    }

    pub fn get_mantle_address(&self) -> &str {
        match self {
            Self::ETH => "0x0000000000000000000000000000000000000000",
            Self::USDC => "0x09bc4e0d864854c6afb6eb9a9cdfe58c4fcaa6e5",
            Self::USDT => "0x201eba5cc46d216ce6dc03f6a759e8e766e956ae",
            Self::WETH => "0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111",
            Self::DAI => "0x0000000000000000000000000000000000000000",
            Self::MNT => "0xdeaddeaddeaddeaddeaddeaddeaddeaddead0000",
        }
    }

    pub fn get_decimals(&self) -> u8 {
        match self {
            Self::ETH | Self::WETH | Self::MNT => 18,
            Self::USDC | Self::USDT => 6,
            Self::DAI => 18,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Self::ETH => "ETH",
            Self::USDC => "USDC",
            Self::USDT => "USDT",
            Self::WETH => "WETH",
            Self::DAI => "DAI",
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

    fn resolve_token_bridge_info(
        &self,
        source_token: &str,
        amount: &str,
        direction: &BridgeDirection,
    ) -> Result<TokenBridgeInfo> {
        let token_type = TokenType::from_address(source_token)?;

        let (source_address, dest_address) = match direction {
            BridgeDirection::EthereumToMantle => (
                token_type.get_ethereum_address().to_string(),
                token_type.get_mantle_address().to_string(),
            ),
            BridgeDirection::MantleToEthereum => (
                token_type.get_mantle_address().to_string(),
                token_type.get_ethereum_address().to_string(),
            ),
            BridgeDirection::Unknown => return Err(anyhow!("Unknown bridge direction")),
        };

        if dest_address == "0x0000000000000000000000000000000000000000"
            && token_type != TokenType::ETH
        {
            return Err(anyhow!(
                "Token {} not supported on destination chain",
                token_type.symbol()
            ));
        }

        Ok(TokenBridgeInfo {
            token_type,
            source_address,
            dest_address,
            amount: amount.to_string(),
            decimals: token_type.get_decimals(),
        })
    }

    pub async fn start(&self) -> Result<(), String> {
        info!("🌉 Starting multi-token Mantle bridge coordinator");

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

        self.start_merkle_sync_tasks();

        loop {
            if let Err(e) = self.process_pending_intents().await {
                error!("❌ Error processing pending intents: {}", e);
                self.record_error(e.to_string()).await;
            }

            sleep(Duration::from_secs(10)).await;
        }
    }

    fn start_merkle_sync_tasks(&self) {
        let eth_relayer = Arc::clone(&self.ethereum_relayer);
        let mantle_relayer = Arc::clone(&self.mantle_relayer);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = Self::sync_source_to_dest(&eth_relayer, &mantle_relayer, 1).await {
                    error!("Failed to sync Ethereum->Mantle root: {}", e);
                }
            }
        });

        let eth_relayer = Arc::clone(&self.ethereum_relayer);
        let mantle_relayer = Arc::clone(&self.mantle_relayer);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = Self::sync_source_to_dest(&mantle_relayer, &eth_relayer, 5000).await
                {
                    error!("Failed to sync Mantle->Ethereum root: {}", e);
                }
            }
        });
    }

    async fn sync_source_to_dest<S, D>(source: &Arc<S>, dest: &Arc<D>, chain_id: u32) -> Result<()>
    where
        S: ChainRelayer,
        D: ChainRelayer,
    {
        let root = source.get_merkle_root().await?;
        dest.sync_source_chain_root(chain_id, root).await?;
        Ok(())
    }

    async fn process_pending_intents(&self) -> Result<()> {
        let pending_intents = self
            .database
            .get_pending_intents()
            .map_err(|e| anyhow!("Failed to get pending intents: {}", e))?;

        if pending_intents.is_empty() {
            debug!("No pending intents to process");
            return Ok(());
        }

        for intent in pending_intents {
            debug!(
                "🔄 Processing intent: {} (status: {:?})",
                intent.id, intent.status
            );

            {
                let mut metrics = self.metrics.write().await;
                metrics.total_intents_processed += 1;
            }

            match intent.status {
                IntentStatus::Created => {
                    if let Err(e) = self.handle_created_intent(&intent).await {
                        error!("Failed to handle created intent {}: {}", intent.id, e);
                    }
                }
                IntentStatus::Filled => {
                    if let Err(e) = self.handle_filled_intent(&intent).await {
                        error!("Failed to handle filled intent {}: {}", intent.id, e);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_created_intent(&self, intent: &Intent) -> Result<()> {
        let direction = self.determine_bridge_direction(intent);
        let token_info =
            self.resolve_token_bridge_info(&intent.source_token, &intent.amount, &direction)?;

        info!(
            "📦 Processing {} bridge: {} {}",
            token_info.token_type.symbol(),
            token_info.amount,
            match direction {
                BridgeDirection::EthereumToMantle => "ETH→Mantle",
                BridgeDirection::MantleToEthereum => "Mantle→ETH",
                BridgeDirection::Unknown => "Unknown",
            }
        );

        match direction {
            BridgeDirection::EthereumToMantle => {
                self.handle_ethereum_to_mantle(intent, &token_info).await?;
            }
            BridgeDirection::MantleToEthereum => {
                self.handle_mantle_to_ethereum(intent, &token_info).await?;
            }
            BridgeDirection::Unknown => {
                warn!("Unknown bridge direction for intent {}", intent.id);
            }
        }

        Ok(())
    }

    async fn handle_ethereum_to_mantle(
        &self,
        intent: &Intent,
        token_info: &TokenBridgeInfo,
    ) -> Result<()> {
        if intent.source_commitment.is_none() {
            return Err(anyhow!("Intent not yet committed on Ethereum"));
        }

        if intent.dest_fill_txid.is_none() {
            info!(
                "🔨 Filling {} intent on Mantle for {}",
                token_info.token_type.symbol(),
                intent.id
            );

            let privacy_params = self
                .database
                .get_intent_privacy_params(&intent.id)
                .map_err(|e| anyhow!("Failed to get privacy params: {}", e))?;

            let commitment = privacy_params
                .commitment
                .ok_or_else(|| anyhow!("Missing commitment"))?;

            let source_root = self.ethereum_relayer.get_merkle_root().await?;

            let merkle_proof = self
                .merkle_tree_manager
                .generate_ethereum_proof(&intent.id)
                .await?;

            let result = self
                .mantle_relayer
                .fill_intent(
                    &intent.id,
                    &commitment,
                    1,
                    &token_info.dest_address,
                    &intent.amount,
                    &source_root,
                    &merkle_proof.path,
                    merkle_proof
                        .leaf_index
                        .try_into()
                        .map_err(|_| anyhow!("Leaf index too large for u32"))?,
                )
                .await;

            match result {
                Ok(txid) => {
                    self.database
                        .update_dest_fill_txid(&intent.id, &txid)
                        .map_err(|e| anyhow!("Failed to update dest txid: {}", e))?;

                    self.database
                        .update_intent_status(&intent.id, IntentStatus::Filled)
                        .map_err(|e| anyhow!("Failed to update status: {}", e))?;

                    let mut metrics = self.metrics.write().await;
                    metrics.mantle_fills += 1;

                    let volume = intent.amount.parse::<u128>().unwrap_or(0);
                    *metrics
                        .volumes_by_token
                        .entry(token_info.token_type.clone())
                        .or_insert(0) += volume;

                    info!(
                        "✅ {} intent filled on Mantle: {}",
                        token_info.token_type.symbol(),
                        txid
                    );
                }
                Err(e) => {
                    error!(
                        "❌ Failed to fill {} intent on Mantle: {}",
                        token_info.token_type.symbol(),
                        e
                    );
                    return Err(anyhow!("Mantle fill failed: {}", e));
                }
            }
        }

        Ok(())
    }

    async fn handle_mantle_to_ethereum(
        &self,
        intent: &Intent,
        token_info: &TokenBridgeInfo,
    ) -> Result<()> {
        if intent.source_commitment.is_none() {
            return Err(anyhow!("Intent not yet committed on Mantle"));
        }

        if intent.dest_fill_txid.is_none() {
            info!(
                "🔨 Filling {} intent on Ethereum for {}",
                token_info.token_type.symbol(),
                intent.id
            );

            let privacy_params = self
                .database
                .get_intent_privacy_params(&intent.id)
                .map_err(|e| anyhow!("Failed to get privacy params: {}", e))?;

            let commitment = privacy_params
                .commitment
                .ok_or_else(|| anyhow!("Missing commitment"))?;

            let source_root = self.mantle_relayer.get_merkle_root().await?;

            let merkle_proof = self
                .merkle_tree_manager
                .generate_mantle_proof(&intent.id)
                .await?;

            let result = self
                .ethereum_relayer
                .fill_intent(
                    &intent.id,
                    &commitment,
                    5000,
                    &token_info.dest_address,
                    &intent.amount,
                    &source_root,
                    &merkle_proof.path,
                    merkle_proof
                        .leaf_index
                        .try_into()
                        .map_err(|_| anyhow!("Leaf index too large for u32"))?,
                )
                .await;

            match result {
                Ok(txid) => {
                    self.database
                        .update_dest_fill_txid(&intent.id, &txid)
                        .map_err(|e| anyhow!("Failed to update dest txid: {}", e))?;

                    self.database
                        .update_intent_status(&intent.id, IntentStatus::Filled)
                        .map_err(|e| anyhow!("Failed to update status: {}", e))?;

                    let mut metrics = self.metrics.write().await;
                    metrics.ethereum_fills += 1;

                    let volume = intent.amount.parse::<u128>().unwrap_or(0);
                    *metrics
                        .volumes_by_token
                        .entry(token_info.token_type.clone())
                        .or_insert(0) += volume;

                    info!(
                        "✅ {} intent filled on Ethereum: {}",
                        token_info.token_type.symbol(),
                        txid
                    );
                }
                Err(e) => {
                    error!(
                        "❌ Failed to fill {} intent on Ethereum: {}",
                        token_info.token_type.symbol(),
                        e
                    );
                    return Err(anyhow!("Ethereum fill failed: {}", e));
                }
            }
        }

        Ok(())
    }

    async fn handle_filled_intent(&self, intent: &Intent) -> Result<()> {
        let direction = self.determine_bridge_direction(intent);
        let token_info =
            self.resolve_token_bridge_info(&intent.source_token, &intent.amount, &direction)?;

        let now = chrono::Utc::now().timestamp() as u64;
        if now > intent.deadline {
            info!("⏰ Intent {} expired, initiating refund", intent.id);
            return self.handle_refund(intent, &token_info).await;
        }

        match direction {
            BridgeDirection::EthereumToMantle => {
                self.claim_on_mantle(intent, &token_info).await?;
                self.mark_source_filled_on_ethereum(intent, &token_info)
                    .await?;
            }
            BridgeDirection::MantleToEthereum => {
                self.claim_on_ethereum(intent, &token_info).await?;
                self.mark_source_filled_on_mantle(intent, &token_info)
                    .await?;
            }
            BridgeDirection::Unknown => {
                return Err(anyhow!("Cannot claim intent with unknown direction"));
            }
        }

        Ok(())
    }

    async fn claim_on_mantle(&self, intent: &Intent, token_info: &TokenBridgeInfo) -> Result<()> {
        let privacy_params = self
            .database
            .get_intent_privacy_params(&intent.id)
            .map_err(|e| anyhow!("Failed to get privacy params: {}", e))?;

        let secret = privacy_params
            .secret
            .as_ref()
            .ok_or_else(|| anyhow!("Secret not available"))?;

        let nullifier = privacy_params
            .nullifier
            .as_ref()
            .ok_or_else(|| anyhow!("Nullifier not available"))?;

        let recipient = privacy_params
            .recipient
            .as_ref()
            .ok_or_else(|| anyhow!("Recipient not available"))?;

        let claim_auth = privacy_params
            .claim_signature
            .as_ref()
            .ok_or_else(|| anyhow!("Claim signature not available"))?;

        let result = self
            .mantle_relayer
            .claim_withdrawal(
                &intent.id,
                nullifier,
                recipient,
                secret,
                claim_auth.as_bytes(),
            )
            .await;

        drop(privacy_params);

        match result {
            Ok(txid) => {
                info!(
                    "✅ Claimed {} on Mantle: {}",
                    token_info.token_type.symbol(),
                    txid
                );
                let mut metrics = self.metrics.write().await;
                metrics.mantle_claims += 1;
                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ Mantle claim failed for {}: {}",
                    token_info.token_type.symbol(),
                    e
                );
                Err(anyhow!("Mantle claim failed: {}", e))
            }
        }
    }

    async fn claim_on_ethereum(&self, intent: &Intent, token_info: &TokenBridgeInfo) -> Result<()> {
        // Fetch secret just-in-time (not from parameters)
        let privacy_params = self
            .database
            .get_intent_privacy_params(&intent.id)
            .map_err(|e| anyhow!("Failed to get privacy params: {}", e))?;

        let secret = privacy_params
            .secret
            .as_ref()
            .ok_or_else(|| anyhow!("Secret not available"))?;

        let nullifier = privacy_params
            .nullifier
            .as_ref()
            .ok_or_else(|| anyhow!("Nullifier not available"))?;

        let recipient = privacy_params
            .recipient
            .as_ref()
            .ok_or_else(|| anyhow!("Recipient not available"))?;

        let claim_auth = privacy_params
            .claim_signature
            .as_ref()
            .ok_or_else(|| anyhow!("Claim signature not available"))?;

        let result = self
            .ethereum_relayer
            .claim_withdrawal(
                &intent.id,
                nullifier,
                recipient,
                secret,
                claim_auth.as_bytes(),
            )
            .await;

        // Secret dropped immediately after use
        drop(privacy_params);

        match result {
            Ok(txid) => {
                info!(
                    "✅ Claimed {} on Ethereum: {}",
                    token_info.token_type.symbol(),
                    txid
                );
                let mut metrics = self.metrics.write().await;
                metrics.ethereum_claims += 1;
                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ Ethereum claim failed for {}: {}",
                    token_info.token_type.symbol(),
                    e
                );
                Err(anyhow!("Ethereum claim failed: {}", e))
            }
        }
    }

    async fn mark_source_filled_on_ethereum(
        &self,
        intent: &Intent,
        token_info: &TokenBridgeInfo,
    ) -> Result<()> {
        let dest_root = self.mantle_relayer.get_merkle_root().await?;

        let merkle_proof = self
            .merkle_tree_manager
            .generate_mantle_proof(&intent.source_commitment.as_ref().unwrap())
            .await?;

        let result = self
            .ethereum_relayer
            .mark_filled(
                &intent.id,
                &merkle_proof.path,
                merkle_proof
                    .leaf_index
                    .try_into()
                    .map_err(|_| anyhow!("Leaf index too large for u32"))?,
            )
            .await;

        match result {
            Ok(txid) => {
                info!(
                    "✅ Marked {} filled on Ethereum: {}",
                    token_info.token_type.symbol(),
                    txid
                );
                self.database
                    .update_intent_status(&intent.id, IntentStatus::Completed)
                    .map_err(|e| anyhow!("Failed to update status: {}", e))?;

                let mut metrics = self.metrics.write().await;
                metrics.successful_bridges += 1;
            }
            Err(e) => {
                error!(
                    "❌ Failed to mark {} filled on Ethereum: {}",
                    token_info.token_type.symbol(),
                    e
                );
                return Err(anyhow!("Mark filled failed: {}", e));
            }
        }

        Ok(())
    }

    async fn mark_source_filled_on_mantle(
        &self,
        intent: &Intent,
        token_info: &TokenBridgeInfo,
    ) -> Result<()> {
        let dest_root = self.ethereum_relayer.get_merkle_root().await?;

        let merkle_proof = self
            .merkle_tree_manager
            .generate_ethereum_proof(&intent.source_commitment.as_ref().unwrap())
            .await?;

        let result = self
            .mantle_relayer
            .mark_filled(
                &intent.id,
                &merkle_proof.path,
                merkle_proof
                    .leaf_index
                    .try_into()
                    .map_err(|_| anyhow!("Leaf index too large for u32"))?,
            )
            .await;

        match result {
            Ok(txid) => {
                info!(
                    "✅ Marked {} filled on Mantle: {}",
                    token_info.token_type.symbol(),
                    txid
                );
                self.database
                    .update_intent_status(&intent.id, IntentStatus::Completed)
                    .map_err(|e| anyhow!("Failed to update status: {}", e))?;

                let mut metrics = self.metrics.write().await;
                metrics.successful_bridges += 1;
            }
            Err(e) => {
                error!(
                    "❌ Failed to mark {} filled on Mantle: {}",
                    token_info.token_type.symbol(),
                    e
                );
                return Err(anyhow!("Mark filled failed: {}", e));
            }
        }

        Ok(())
    }

    async fn handle_refund(&self, intent: &Intent, token_info: &TokenBridgeInfo) -> Result<()> {
        let direction = self.determine_bridge_direction(intent);

        match direction {
            BridgeDirection::EthereumToMantle => {
                self.ethereum_relayer
                    .refund_intent(&intent.id)
                    .await
                    .map_err(|e| anyhow!("Ethereum refund failed: {}", e))?;
            }
            BridgeDirection::MantleToEthereum => {
                self.mantle_relayer
                    .refund_intent(&intent.id)
                    .await
                    .map_err(|e| anyhow!("Mantle refund failed: {}", e))?;
            }
            BridgeDirection::Unknown => {}
        }

        self.database
            .update_intent_status(&intent.id, IntentStatus::Refunded)
            .map_err(|e| anyhow!("Failed to update status: {}", e))?;

        let mut metrics = self.metrics.write().await;
        metrics.refunded_intents += 1;

        info!(
            "♻️ {} intent {} refunded",
            token_info.token_type.symbol(),
            intent.id
        );
        Ok(())
    }

    fn determine_bridge_direction(&self, intent: &Intent) -> BridgeDirection {
        match intent.dest_chain.as_str() {
            "mantle" => BridgeDirection::EthereumToMantle,
            "ethereum" => BridgeDirection::MantleToEthereum,
            _ => BridgeDirection::Unknown,
        }
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
                    1 => token_type.get_ethereum_address(),
                    5000 => token_type.get_mantle_address(),
                    _ => return false,
                };
                dest_address != "0x0000000000000000000000000000000000000000"
                    || token_type == TokenType::ETH
            })
            .unwrap_or(false)
    }
}
