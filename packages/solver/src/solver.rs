use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use crate::model::{
    ActiveFill, DetectedIntent, FillOpportunity, FillStatus, SolverConfig, SolverMetrics,
    SupportedToken,
};
use anyhow::{Context, Result, anyhow};
use ethers::{
    contract::abigen,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{LocalWallet, Signer, Wallet},
    types::{Address, Filter, H256, Log, U256},
};
use tokio::{sync::RwLock, time::interval};
use tracing::{debug, error, info, warn};

abigen!(
    SettlementContract,
    r#"[
            function fillIntent(bytes32 intentId, bytes32 commitment, uint32 sourceChain, address token, uint256 amount) external payable
            function getMerkleRoot() external view returns (bytes32)
            function getIntentParams(bytes32 intentId) external view returns (tuple(bytes32 commitment, address token, uint256 amount, uint32 sourceChain, uint64 deadline, bool exists))
            function getFill(bytes32 intentId) external view returns (tuple(address solver, address token, uint256 amount, uint32 sourceChain, uint32 timestamp, bool claimed))
            function isTokenSupported(address token) external view returns (bool)
            function generateFillProof(bytes32 intentId) external view returns (bytes32[] memory)
            function getFillTreeSize() external view returns (uint256)
            event IntentRegistered(bytes32 indexed intentId, bytes32 commitment, address token, uint256 amount, uint32 sourceChain, uint64 deadline, bytes32[] proof, uint256 leafIndex)
            event IntentFilled(bytes32 indexed intentId, address indexed solver, address indexed token, uint256 amount)
            event WithdrawalClaimed(bytes32 indexed intentId, bytes32 indexed nullifier, address recipient, address token, uint256 amount)

    ]"#
);

abigen!(
    IntentPoolContract,
    r#"[
        function generateCommitmentProof(bytes32 commitment) external view returns (bytes32[] memory, uint256)
        function markFilled(bytes32 intentId, bytes32[] calldata merkleProof, uint256 leafIndex) external
        event IntentCreated(bytes32 indexed intentId, bytes32 indexed commitment, uint32 destChain, uint256 amount, address token)
        event IntentFilled(bytes32 indexed intentId, address indexed solver, uint256 amount)
    ]"#
);

abigen!(
    ERC20Contract,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function allowance(address owner, address spender) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function decimals() external view returns (uint8)
        function symbol() external view returns (string)
    ]"#
);

impl SupportedToken {
    pub fn address(&self, chain_id: u64) -> Address {
        match (self, chain_id) {
            // Ethereum Sepolia (11155111)
            (Self::ETH, 11155111) => {
                Address::from_str("0x0000000000000000000000000000000000000000").unwrap()
            }
            (Self::WETH, 11155111) => {
                Address::from_str("0x50e8Da97BeEB8064714dE45ce1F250879f3bD5B5").unwrap()
            }
            (Self::USDC, 11155111) => {
                Address::from_str("0x28650373758d75a8fF0B22587F111e47BAC34e21").unwrap()
            }
            (Self::USDT, 11155111) => {
                Address::from_str("0x89F4f0e13997Ca27cEB963DEE291C607e4E59923").unwrap()
            }
            (Self::MNT, 11155111) => {
                Address::from_str("0x65e37B558F64E2Be5768DB46DF22F93d85741A9E").unwrap()
            }

            // Mantle Sepolia (5003)
            (Self::MNT, 5003) => {
                Address::from_str("0x44FCE297e4D6c5A50D28Fb26A58202e4D49a13E7").unwrap()
            }
            (Self::WETH, 5003) => {
                Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111").unwrap()
            }
            (Self::USDC, 5003) => {
                Address::from_str("0xA4b184006B59861f80521649b14E4E8A72499A23").unwrap()
            }
            (Self::USDT, 5003) => {
                Address::from_str("0xB0ee6EF7788E9122fc4AAE327Ed4FEf56c7da891").unwrap()
            }

            _ => Address::zero(),
        }
    }

    pub fn decimals(&self) -> u8 {
        match self {
            Self::ETH | Self::WETH | Self::MNT => 18,
            Self::USDC | Self::USDT => 6,
        }
    }

    pub fn min_amount(&self) -> U256 {
        match self {
            Self::ETH | Self::WETH | Self::MNT => U256::from(10).pow(U256::from(15)), // 0.001
            Self::USDC | Self::USDT => U256::from(10).pow(U256::from(6)),             // 1 USDC/USDT
        }
    }

    pub fn max_amount(&self) -> U256 {
        match self {
            Self::ETH | Self::WETH | Self::MNT => {
                U256::from(100) * U256::from(10).pow(U256::from(18))
            }
            Self::USDC | Self::USDT => U256::from(100000) * U256::from(10).pow(U256::from(6)),
        }
    }

    pub fn is_native(&self) -> bool {
        matches!(self, Self::ETH | Self::MNT)
    }
}

impl FromStr for SupportedToken {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "ETH" => Ok(Self::ETH),
            "USDC" => Ok(Self::USDC),
            "USDT" => Ok(Self::USDT),
            "WETH" => Ok(Self::WETH),
            "MNT" => Ok(Self::MNT),
            _ => Err(anyhow!("Unsupported token: {}", s)),
        }
    }
}

impl Default for SolverConfig {
    fn default() -> Self {
        let mut max_capital = HashMap::new();
        max_capital.insert(SupportedToken::ETH, U256::from(10) * U256::exp10(18));
        max_capital.insert(SupportedToken::WETH, U256::from(10) * U256::exp10(18));
        max_capital.insert(SupportedToken::MNT, U256::from(1000) * U256::exp10(18));
        max_capital.insert(SupportedToken::USDC, U256::from(10000) * U256::exp10(6));
        max_capital.insert(SupportedToken::USDT, U256::from(10000) * U256::exp10(6));

        let mut min_reserve = HashMap::new();
        min_reserve.insert(SupportedToken::ETH, U256::from(1) * U256::exp10(18));
        min_reserve.insert(SupportedToken::WETH, U256::from(1) * U256::exp10(18));
        min_reserve.insert(SupportedToken::MNT, U256::from(100) * U256::exp10(18));
        min_reserve.insert(SupportedToken::USDC, U256::from(1000) * U256::exp10(6));
        min_reserve.insert(SupportedToken::USDT, U256::from(1000) * U256::exp10(6));

        Self {
            max_capital_per_fill: max_capital,
            min_capital_reserve: min_reserve,
            max_concurrent_fills: 10,
            min_profit_bps: 10,
            source_confirmations_required: 12,
            max_intent_age_secs: 3600,
            ethereum_rpc: String::new(),
            mantle_rpc: String::new(),
            ethereum_settlement: Address::zero(),
            mantle_settlement: Address::zero(),
            ethereum_intent_pool: Address::zero(),
            mantle_intent_pool: Address::zero(),
            ethereum_chain_id: 11155111,
            mantle_chain_id: 5003,
            solver_address: Address::zero(),
            solver_private_key: String::new(),
            max_gas_price_gwei: U256::from(50),
            priority_fee_gwei: U256::from(2),
            health_check_interval_secs: 30,
            balance_check_interval_secs: 60,
        }
    }
}

pub struct CrossChainSolver {
    pub config: SolverConfig,
    ethereum_provider: Arc<Provider<Ws>>,
    mantle_provider: Arc<Provider<Ws>>,
    ethereum_client: Arc<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,
    mantle_client: Arc<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,

    // Contract instances
    ethereum_settlement:
        SettlementContract<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,
    mantle_settlement: SettlementContract<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,
    ethereum_intent_pool:
        IntentPoolContract<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,
    mantle_intent_pool: IntentPoolContract<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,

    // State Management
    active_fills: Arc<RwLock<HashMap<H256, ActiveFill>>>,
    processed_intents: Arc<RwLock<HashMap<H256, bool>>>,
    metrics: Arc<RwLock<SolverMetrics>>,

    // Token Balances Cache
    token_balances: Arc<RwLock<HashMap<(SupportedToken, u64), U256>>>,
}

impl CrossChainSolver {
    pub async fn new(config: SolverConfig) -> Result<Self> {
        info!("🚀 Initializing CrossChainSolver");

        let ethereum_provider = Arc::new(
            Provider::<Ws>::connect(&config.ethereum_rpc)
                .await
                .context("Failed to connect to Ethereum")?,
        );

        let mantle_provider = Arc::new(
            Provider::<Ws>::connect(&config.mantle_rpc)
                .await
                .context("Failed to connect to Mantle")?,
        );

        let ethereum_wallet = config
            .solver_private_key
            .parse::<LocalWallet>()?
            .with_chain_id(config.ethereum_chain_id);

        let mantle_wallet = config
            .solver_private_key
            .parse::<LocalWallet>()?
            .with_chain_id(config.mantle_chain_id);

        let ethereum_client = Arc::new(SignerMiddleware::new(
            ethereum_provider.clone(),
            ethereum_wallet,
        ));

        let mantle_client = Arc::new(SignerMiddleware::new(
            mantle_provider.clone(),
            mantle_wallet,
        ));

        let ethereum_settlement =
            SettlementContract::new(config.ethereum_settlement, ethereum_client.clone());

        let mantle_settlement =
            SettlementContract::new(config.mantle_settlement, mantle_client.clone());

        let ethereum_intent_pool =
            IntentPoolContract::new(config.ethereum_intent_pool, ethereum_client.clone());

        let mantle_intent_pool =
            IntentPoolContract::new(config.mantle_intent_pool, mantle_client.clone());

        info!(
            "✅ Solver initialized with address: {:?}",
            config.solver_address
        );

        Ok(Self {
            config,
            ethereum_provider,
            mantle_provider,
            ethereum_client,
            mantle_client,
            ethereum_settlement,
            mantle_settlement,
            ethereum_intent_pool,
            mantle_intent_pool,
            active_fills: Arc::new(RwLock::new(HashMap::new())),
            processed_intents: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(SolverMetrics::default())),
            token_balances: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        info!("🏃 Starting solver main loop");

        let health_monitor = Arc::clone(&self);
        tokio::spawn(async move {
            if let Err(e) = health_monitor.run_health_checks().await {
                error!("Health monitor error: {}", e);
            }
        });

        let balance_monitor = Arc::clone(&self);
        tokio::spawn(async move {
            if let Err(e) = balance_monitor.monitor_balances().await {
                error!("Balance monitor error: {}", e);
            }
        });

        let fill_monitor = Arc::clone(&self);
        tokio::spawn(async move {
            if let Err(e) = fill_monitor.monitor_active_fills().await {
                error!("Fill monitor error: {}", e);
            }
        });

        tokio::try_join!(
            self.clone().monitor_ethereum_registered_intents(),
            self.clone().monitor_mantle_registered_intents(),
        )?;

        Ok(())
    }

    /// Monitor Ethereum IntentPool for new opportunities
    async fn monitor_ethereum_registered_intents(self: Arc<Self>) -> Result<()> {
        info!("👀 Monitoring Ethereum Settlement IntentRegistered events (polling mode)");

        let filter = Filter::new()
            .address(self.config.ethereum_settlement)
            .event(
                "IntentRegistered(bytes32,bytes32,address,uint256,uint32,uint64,bytes32[],uint256)",
            );

        let mut last_block = self.ethereum_provider.get_block_number().await?.as_u64();
        let mut poll_interval = interval(Duration::from_secs(12)); // Ethereum block time

        loop {
            poll_interval.tick().await;

            let current_block = match self.ethereum_provider.get_block_number().await {
                Ok(block) => block.as_u64(),
                Err(e) => {
                    warn!("⚠️ Failed to get Ethereum block number: {}", e);
                    continue;
                }
            };

            if current_block <= last_block {
                continue;
            }

            let logs = match self
                .ethereum_provider
                .get_logs(
                    &filter
                        .clone()
                        .from_block(last_block + 1)
                        .to_block(current_block),
                )
                .await
            {
                Ok(logs) => logs,
                Err(e) => {
                    warn!("⚠️ Failed to fetch Ethereum logs: {}", e);
                    continue;
                }
            };

            for log in logs {
                if let Err(e) = self
                    .handle_registered_intent(log, self.config.ethereum_chain_id as u32)
                    .await
                {
                    error!("❌ Error handling registered intent: {}", e);
                    self.record_error(e.to_string()).await;
                }
            }

            last_block = current_block;
        }
    }

    async fn monitor_mantle_registered_intents(self: Arc<Self>) -> Result<()> {
        info!("👀 Monitoring Mantle Settlement IntentRegistered events (polling mode)");

        let filter = Filter::new().address(self.config.mantle_settlement).event(
            "IntentRegistered(bytes32,bytes32,address,uint256,uint32,uint64,bytes32[],uint256)",
        );

        let mut last_block = self.mantle_provider.get_block_number().await?.as_u64();
        let mut poll_interval = interval(Duration::from_secs(3)); // Mantle ~3s block time

        loop {
            poll_interval.tick().await;

            let current_block = match self.mantle_provider.get_block_number().await {
                Ok(block) => block.as_u64(),
                Err(e) => {
                    warn!("⚠️ Failed to get Mantle block number: {}", e);
                    continue;
                }
            };

            if current_block <= last_block {
                continue;
            }

            let logs = match self
                .mantle_provider
                .get_logs(
                    &filter
                        .clone()
                        .from_block(last_block + 1)
                        .to_block(current_block),
                )
                .await
            {
                Ok(logs) => logs,
                Err(e) => {
                    warn!("⚠️ Failed to fetch Mantle logs: {}", e);
                    continue;
                }
            };

            for log in logs {
                if let Err(e) = self
                    .handle_registered_intent(log, self.config.mantle_chain_id as u32)
                    .await
                {
                    error!("❌ Error handling registered intent: {}", e);
                    self.record_error(e.to_string()).await;
                }
            }

            last_block = current_block;
        }
    }

    async fn handle_registered_intent(&self, log: Log, dest_chain: u32) -> Result<()> {
        let event = self
            .mantle_settlement
            .decode_event::<IntentRegisteredFilter>(
                "IntentRegistered",
                log.topics.clone(),
                log.data.clone(),
            )
            .context("Failed to decode IntentRegistered event")?;

        let intent = DetectedIntent {
            intent_id: H256::from(event.intent_id),
            commitment: H256::from(event.commitment),
            token: event.token,
            token_type: self.identify_token(event.token, dest_chain as u64)?,
            amount: event.amount,
            source_chain: event.source_chain,
            dest_chain,
            source_block: log.block_number.unwrap().as_u64(),
            detected_at: chrono::Utc::now().timestamp() as u64,
        };

        debug!(
            "📋 Intent registered and ready to fill: {:?}",
            intent.intent_id
        );

        // Check deadline
        let deadline = event.deadline;
        let now = chrono::Utc::now().timestamp() as u64;
        if deadline <= now {
            warn!("⏰ Intent already expired");
            return Ok(());
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.total_intents_evaluated += 1;
        }

        {
            let processed = self.processed_intents.read().await;
            if processed.contains_key(&intent.intent_id) {
                debug!("⏭️ Intent already processed");
                return Ok(());
            }
        }

        // Evaluate and fill
        let opportunity = self.evaluate_fill_opportunity(&intent).await?;
        debug!(
            "Opportunity evaluated: profit={}, gas={}, profit_bps={}, risk={}",
            opportunity.estimated_profit,
            opportunity.gas_estimate,
            opportunity.profit_bps,
            opportunity.risk_score
        );

        if self.should_fill(&opportunity).await? {
            info!(
                "💰 Filling registered intent: {:?}, profit: {} bps",
                intent.intent_id, opportunity.profit_bps
            );

            if dest_chain == self.config.mantle_chain_id as u32 {
                self.execute_fill_on_mantle(&intent, &opportunity).await?;
            } else {
                self.execute_fill_on_ethereum(&intent, &opportunity).await?;
            }
        }

        {
            let mut processed = self.processed_intents.write().await;
            processed.insert(intent.intent_id, true);
        }

        Ok(())
    }

    async fn execute_fill_on_mantle(
        &self,
        intent: &DetectedIntent,
        opportunity: &FillOpportunity,
    ) -> Result<()> {
        info!("🔨 Executing fill on Mantle: {:?}", intent.intent_id);

        // Approve token if ERC20
        if !intent.token_type.is_native() {
            self.approve_token_if_needed(
                intent.token,
                self.config.mantle_settlement,
                intent.amount,
                self.mantle_client.clone(),
            )
            .await?;
        }

        let call = self.mantle_settlement.fill_intent(
            intent.intent_id.into(),
            intent.commitment.into(),
            intent.source_chain,
            intent.token,
            intent.amount,
        );

        let tx = if intent.token_type.is_native() {
            call.value(intent.amount)
        } else {
            call
        };

        let pending_tx = tx.send().await.context("Failed to send fillIntent tx")?;
        let tx_hash = pending_tx.tx_hash();

        info!("✅ Fill tx sent: {:?}", tx_hash);

        // Record active fill
        {
            let mut active = self.active_fills.write().await;
            active.insert(
                intent.intent_id,
                ActiveFill {
                    intent_id: intent.intent_id,
                    tx_hash,
                    amount: intent.amount,
                    token: intent.token,
                    token_type: intent.token_type,
                    filled_at: chrono::Utc::now().timestamp() as u64,
                    confirmed_at: None,
                    status: FillStatus::Pending,
                    dest_chain: self.config.mantle_chain_id as u32,
                },
            );
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.total_fills_attempted += 1;
            *metrics
                .capital_deployed
                .entry(intent.token_type)
                .or_insert(U256::zero()) += opportunity.capital_required;
            metrics.active_fills_count += 1;
        }

        // Wait for confirmation
        match pending_tx.await? {
            Some(receipt) => {
                if receipt.status == Some(0.into()) {
                    error!("❌ Fill tx reverted: {:?}", tx_hash);

                    let mut active = self.active_fills.write().await;
                    if let Some(fill) = active.get_mut(&intent.intent_id) {
                        fill.status = FillStatus::Failed;
                    }

                    let mut metrics = self.metrics.write().await;
                    metrics.failed_fills += 1;
                    metrics.active_fills_count = metrics.active_fills_count.saturating_sub(1);

                    return Err(anyhow!("Transaction reverted"));
                }

                info!(
                    "✅ Fill confirmed in block: {}",
                    receipt.block_number.unwrap()
                );

                let mut active = self.active_fills.write().await;
                if let Some(fill) = active.get_mut(&intent.intent_id) {
                    fill.status = FillStatus::Confirmed;
                    fill.confirmed_at = Some(chrono::Utc::now().timestamp() as u64);
                }
            }
            None => {
                error!("❌ Fill tx dropped from mempool: {:?}", tx_hash);

                let mut active = self.active_fills.write().await;
                if let Some(fill) = active.get_mut(&intent.intent_id) {
                    fill.status = FillStatus::Failed;
                }

                return Err(anyhow!("Transaction dropped"));
            }
        }

        Ok(())
    }

    async fn execute_fill_on_ethereum(
        &self,
        intent: &DetectedIntent,
        opportunity: &FillOpportunity,
    ) -> Result<()> {
        info!("🔨 Executing fill on Ethereum: {:?}", intent.intent_id);

        if !intent.token_type.is_native() {
            self.approve_token_if_needed(
                intent.token,
                self.config.ethereum_settlement,
                intent.amount,
                self.ethereum_client.clone(),
            )
            .await?;
        }

        let call = self.ethereum_settlement.fill_intent(
            intent.intent_id.into(),
            intent.commitment.into(),
            intent.source_chain,
            intent.token,
            intent.amount,
        );

        let tx = if intent.token_type.is_native() {
            call.value(intent.amount)
        } else {
            call
        };

        let pending_tx = tx.send().await.context("Failed to send fillIntent tx")?;
        let tx_hash = pending_tx.tx_hash();

        info!("✅ Fill tx sent: {:?}", tx_hash);

        {
            let mut active = self.active_fills.write().await;
            active.insert(
                intent.intent_id,
                ActiveFill {
                    intent_id: intent.intent_id,
                    tx_hash,
                    amount: intent.amount,
                    token: intent.token,
                    token_type: intent.token_type,
                    filled_at: chrono::Utc::now().timestamp() as u64,
                    confirmed_at: None,
                    status: FillStatus::Pending,
                    dest_chain: self.config.ethereum_chain_id as u32,
                },
            );
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.total_fills_attempted += 1;
            *metrics
                .capital_deployed
                .entry(intent.token_type)
                .or_insert(U256::zero()) += opportunity.capital_required;
            metrics.active_fills_count += 1;
        }

        match pending_tx.await? {
            Some(receipt) => {
                if receipt.status == Some(0.into()) {
                    error!("❌ Fill tx reverted on Ethereum: {:?}", tx_hash);

                    let mut active = self.active_fills.write().await;
                    if let Some(fill) = active.get_mut(&intent.intent_id) {
                        fill.status = FillStatus::Failed;
                    }

                    let mut metrics = self.metrics.write().await;
                    metrics.failed_fills += 1;
                    metrics.active_fills_count = metrics.active_fills_count.saturating_sub(1);

                    return Err(anyhow!("Transaction reverted"));
                }

                info!(
                    "✅ Fill confirmed in block: {}",
                    receipt.block_number.unwrap()
                );

                let mut active = self.active_fills.write().await;
                if let Some(fill) = active.get_mut(&intent.intent_id) {
                    fill.status = FillStatus::Confirmed;
                    fill.confirmed_at = Some(chrono::Utc::now().timestamp() as u64);
                }
            }
            None => {
                error!("❌ Fill tx dropped from mempool: {:?}", tx_hash);

                let mut active = self.active_fills.write().await;
                if let Some(fill) = active.get_mut(&intent.intent_id) {
                    fill.status = FillStatus::Failed;
                }

                return Err(anyhow!("Transaction dropped"));
            }
        }

        Ok(())
    }

    async fn claim_solver_rewards(&self, fill: &ActiveFill) -> Result<()> {
        info!(
            "💰 Claiming solver rewards for intent: {:?}",
            fill.intent_id
        );

        // Get fill proof from destination chain settlement contract
        let fill_proof = if fill.dest_chain == self.config.ethereum_chain_id as u32 {
            self.ethereum_settlement
                .generate_fill_proof(fill.intent_id.into())
                .call()
                .await
                .context("Failed to get fill proof from Ethereum")?
        } else {
            self.mantle_settlement
                .generate_fill_proof(fill.intent_id.into())
                .call()
                .await
                .context("Failed to get fill proof from Mantle")?
        };

        // Get fill tree size to calculate leaf index
        let leaf_index = if fill.dest_chain == self.config.ethereum_chain_id as u32 {
            let tree_size = self
                .ethereum_settlement
                .get_fill_tree_size()
                .call()
                .await
                .context("Failed to get tree size")?;
            tree_size.as_u64() - 1
        } else {
            let tree_size = self
                .mantle_settlement
                .get_fill_tree_size()
                .call()
                .await
                .context("Failed to get tree size")?;
            tree_size.as_u64() - 1
        };

        // Call markFilled on source chain
        let tx = if fill.dest_chain == self.config.ethereum_chain_id as u32 {
            self.mantle_intent_pool.mark_filled(
                fill.intent_id.into(),
                fill_proof,
                U256::from(leaf_index),
            )
        } else {
            self.ethereum_intent_pool.mark_filled(
                fill.intent_id.into(),
                fill_proof,
                U256::from(leaf_index),
            )
        };

        let pending_tx = tx.send().await.context("Failed to send markFilled tx")?;
        let tx_hash = pending_tx.tx_hash();

        info!("📤 markFilled tx sent: {:?}", tx_hash);

        // Wait for confirmation
        match pending_tx.await? {
            Some(receipt) => {
                if receipt.status == Some(0.into()) {
                    error!("❌ markFilled tx reverted: {:?}", tx_hash);
                    return Err(anyhow!("markFilled transaction reverted"));
                }

                info!(
                    "✅ Solver rewards claimed successfully in block: {}",
                    receipt.block_number.unwrap()
                );

                // Update fill status
                {
                    let mut active = self.active_fills.write().await;
                    if let Some(f) = active.get_mut(&fill.intent_id) {
                        f.status = FillStatus::Claimed;
                    }
                }

                // Update metrics
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.successful_fills += 1;
                    metrics.active_fills_count = metrics.active_fills_count.saturating_sub(1);

                    // Record profit
                    let profit = fill.amount * U256::from(5) / U256::from(10000);
                    *metrics
                        .total_profit_earned
                        .entry(fill.token_type)
                        .or_insert(U256::zero()) += profit;
                }

                Ok(())
            }
            None => {
                error!("❌ markFilled tx dropped from mempool: {:?}", tx_hash);
                Err(anyhow!("Transaction dropped"))
            }
        }
    }

    /// Evaluate fill opportunity profitability
    async fn evaluate_fill_opportunity(&self, intent: &DetectedIntent) -> Result<FillOpportunity> {
        let settlement_fee_bps = 5u128;
        let total_cost_bps = settlement_fee_bps;

        let fees = intent.amount * U256::from(total_cost_bps) / U256::from(10000);
        let estimated_profit = fees;
        let profit_bps = (estimated_profit * U256::from(10000) / intent.amount).as_u64() as u16;

        let risk_score = self.calculate_risk_score(intent).await?;

        // Estimate gas cost
        let gas_estimate = self.estimate_fill_gas(intent).await?;

        Ok(FillOpportunity {
            intent: intent.clone(),
            estimated_profit,
            profit_bps,
            risk_score,
            capital_required: intent.amount,
            gas_estimate,
        })
    }

    async fn estimate_fill_gas(&self, intent: &DetectedIntent) -> Result<U256> {
        let base_gas = if intent.token_type.is_native() {
            U256::from(100_000) // Native transfer
        } else {
            U256::from(150_000) // ERC20 transfer
        };

        let gas_price = if intent.dest_chain == self.config.ethereum_chain_id as u32 {
            self.ethereum_provider.get_gas_price().await?
        } else {
            self.mantle_provider.get_gas_price().await?
        };

        Ok(base_gas * gas_price)
    }

    /// Calculate risk score
    async fn calculate_risk_score(&self, intent: &DetectedIntent) -> Result<u8> {
        let mut score = 0u8;

        let age_secs = chrono::Utc::now().timestamp() as u64 - intent.detected_at;
        if age_secs > 300 {
            score += 10;
        }
        if age_secs > 900 {
            score += 10;
        }
        if age_secs > 1800 {
            score += 20;
        }

        let max_amount = intent.token_type.max_amount();
        if intent.amount > max_amount / U256::from(2) {
            score += 15;
        }
        if intent.amount > max_amount * U256::from(8) / U256::from(10) {
            score += 25;
        }

        let current_block = self.get_source_block_number(intent.source_chain).await?;
        let confirmations = current_block.saturating_sub(intent.source_block);
        if confirmations < self.config.source_confirmations_required {
            score += 30;
        }

        Ok(score.min(100))
    }

    /// Decide whether to fill opportunity
    async fn should_fill(&self, opportunity: &FillOpportunity) -> Result<bool> {
        if opportunity.profit_bps < self.config.min_profit_bps {
            debug!("❌ Insufficient profit: {} bps", opportunity.profit_bps);
            return Ok(false);
        }

        if opportunity.risk_score > 70 {
            warn!("⚠️ High risk: {}", opportunity.risk_score);
            return Ok(false);
        }

        let metrics = self.metrics.read().await;
        if metrics.active_fills_count >= self.config.max_concurrent_fills {
            debug!("❌ Max concurrent fills reached");
            return Ok(false);
        }
        drop(metrics);

        let max_capital = self
            .config
            .max_capital_per_fill
            .get(&opportunity.intent.token_type)
            .ok_or_else(|| anyhow!("Token not configured"))?;

        if opportunity.capital_required > *max_capital {
            debug!("❌ Exceeds max capital per fill");
            return Ok(false);
        }

        let dest_chain = if opportunity.intent.source_chain == self.config.ethereum_chain_id as u32
        {
            self.config.mantle_chain_id
        } else {
            self.config.ethereum_chain_id
        };

        let balance = self
            .get_token_balance(opportunity.intent.token_type, dest_chain)
            .await?;

        if balance < opportunity.capital_required {
            warn!("❌ Insufficient balance");
            return Ok(false);
        }

        info!(
            "✅ Fill approved: profit={}bps, risk={}",
            opportunity.profit_bps, opportunity.risk_score
        );

        Ok(true)
    }

    async fn approve_token_if_needed(
        &self,
        token: Address,
        spender: Address,
        amount: U256,
        client: Arc<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,
    ) -> Result<()> {
        let erc20 = ERC20Contract::new(token, client.clone());

        let allowance = erc20
            .allowance(self.config.solver_address, spender)
            .call()
            .await
            .context("Failed to check allowance")?;

        if allowance < amount {
            info!("🔓 Approving token: {:?}", token);

            let call = erc20.approve(spender, amount);

            let pending_tx = call
                .send()
                .await
                .context("Failed while waiting for approve tx")?;

            let receipt = pending_tx
                .await
                .context("Failed while waiting for approve tx")?;

            receipt.context("Approve tx failed")?;

            info!("✅ Token approved");
        }

        Ok(())
    }

    /// Monitor active fills for claim events
    async fn monitor_active_fills(self: Arc<Self>) -> Result<()> {
        let mut check_interval = interval(Duration::from_secs(15));

        loop {
            check_interval.tick().await;

            let active_fills: Vec<_> = {
                let fills = self.active_fills.read().await;
                fills.values().cloned().collect()
            };

            for fill in active_fills {
                if fill.status != FillStatus::Confirmed {
                    continue;
                }

                // Check if fill is ready to claim (dest root synced)
                if let Err(e) = self.process_confirmed_fill(&fill).await {
                    error!("❌ Error processing confirmed fill: {}", e);
                }
            }
        }
    }

    async fn process_confirmed_fill(&self, fill: &ActiveFill) -> Result<()> {
        // Wait for some confirmations on destination chain before claiming
        let required_confirmations = 6;

        let current_block = if fill.dest_chain == self.config.ethereum_chain_id as u32 {
            self.ethereum_provider.get_block_number().await?.as_u64()
        } else {
            self.mantle_provider.get_block_number().await?.as_u64()
        };

        // Get fill block number from transaction receipt
        let fill_block = if fill.dest_chain == self.config.ethereum_chain_id as u32 {
            self.ethereum_provider
                .get_transaction_receipt(fill.tx_hash)
                .await?
                .and_then(|r| r.block_number)
                .map(|b| b.as_u64())
                .unwrap_or(0)
        } else {
            self.mantle_provider
                .get_transaction_receipt(fill.tx_hash)
                .await?
                .and_then(|r| r.block_number)
                .map(|b| b.as_u64())
                .unwrap_or(0)
        };

        let confirmations = current_block.saturating_sub(fill_block);

        if confirmations < required_confirmations {
            debug!(
                "⏳ Waiting for more confirmations ({}/{}) for intent: {:?}",
                confirmations, required_confirmations, fill.intent_id
            );
            return Ok(());
        }

        // Claim solver rewards
        info!("🎯 Ready to claim rewards for intent: {:?}", fill.intent_id);
        self.claim_solver_rewards(fill).await?;

        Ok(())
    }

    async fn get_token_balance(&self, token: SupportedToken, chain_id: u64) -> Result<U256> {
        let key = (token, chain_id);

        {
            let balances = self.token_balances.read().await;
            if let Some(balance) = balances.get(&key) {
                return Ok(*balance);
            }
        }

        let balance = self.fetch_balance_with_retry(token, chain_id, 3).await?;

        {
            let mut balances = self.token_balances.write().await;
            balances.insert(key, balance);
        }

        Ok(balance)
    }

    async fn fetch_balance_with_retry(
        &self,
        token: SupportedToken,
        chain_id: u64,
        max_retries: u32,
    ) -> Result<U256> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match self.fetch_balance_inner(token, chain_id).await {
                Ok(balance) => return Ok(balance),
                Err(e) => {
                    warn!(
                        "Balance fetch attempt {}/{} failed for {:?} on chain {}: {}",
                        attempt + 1,
                        max_retries,
                        token,
                        chain_id,
                        e
                    );
                    last_error = Some(e);

                    if attempt < max_retries - 1 {
                        tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| anyhow!("Balance fetch failed after {} retries", max_retries)))
    }

    async fn fetch_balance_inner(&self, token: SupportedToken, chain_id: u64) -> Result<U256> {
        if token.is_native() {
            let provider = if chain_id == self.config.ethereum_chain_id {
                &self.ethereum_provider
            } else {
                &self.mantle_provider
            };

            provider
                .get_balance(self.config.solver_address, None)
                .await
                .context("Failed to get native balance")
        } else {
            let client = if chain_id == self.config.ethereum_chain_id {
                self.ethereum_client.clone()
            } else {
                self.mantle_client.clone()
            };

            let erc20 = ERC20Contract::new(token.address(chain_id), client);

            erc20
                .balance_of(self.config.solver_address)
                .call()
                .await
                .context(format!("Failed to get ERC20 balance for {:?}", token))
        }
    }

    async fn get_source_block_number(&self, chain_id: u32) -> Result<u64> {
        let block = if chain_id == self.config.ethereum_chain_id as u32 {
            self.ethereum_provider.get_block_number().await?
        } else {
            self.mantle_provider.get_block_number().await?
        };

        Ok(block.as_u64())
    }

    /// Monitor balances across chains
    async fn monitor_balances(&self) -> Result<()> {
        let mut check_interval =
            interval(Duration::from_secs(self.config.balance_check_interval_secs));

        loop {
            check_interval.tick().await;

            if let Err(e) = self.update_all_balances().await {
                error!("❌ Failed to update balances: {}", e);
            }
        }
    }

    async fn update_all_balances(&self) -> Result<()> {
        for token in [
            SupportedToken::ETH,
            SupportedToken::WETH,
            SupportedToken::USDC,
            SupportedToken::USDT,
            SupportedToken::MNT,
        ] {
            for chain_id in [self.config.ethereum_chain_id, self.config.mantle_chain_id] {
                let balance = self.get_token_balance(token, chain_id).await?;

                debug!("💰 Balance {:?} on chain {}: {}", token, chain_id, balance);

                // Update metrics
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.capital_available.insert((token, chain_id), balance);
                }
            }
        }

        Ok(())
    }

    fn identify_token(&self, token: Address, chain_id: u64) -> Result<SupportedToken> {
        for supported in [
            SupportedToken::ETH,
            SupportedToken::USDC,
            SupportedToken::USDT,
            SupportedToken::WETH,
            SupportedToken::MNT,
        ] {
            if supported.address(chain_id) == token {
                return Ok(supported);
            }
        }
        Err(anyhow!("Unsupported token: {:?}", token))
    }

    /// Run health checks
    async fn run_health_checks(&self) -> Result<()> {
        let mut check_interval =
            interval(Duration::from_secs(self.config.health_check_interval_secs));

        loop {
            check_interval.tick().await;

            if let Err(e) = self.perform_health_check().await {
                error!("❌ Health check failed: {}", e);
            }
        }
    }

    async fn perform_health_check(&self) -> Result<()> {
        let eth_block = self.ethereum_provider.get_block_number().await?;
        let mantle_block = self.mantle_provider.get_block_number().await?;

        debug!(
            "💓 Health: ETH block={}, Mantle block={}",
            eth_block, mantle_block
        );

        let metrics = self.metrics.read().await;

        for ((token, chain_id), balance) in &metrics.capital_available {
            if let Some(min_reserve) = self.config.min_capital_reserve.get(token) {
                if balance < min_reserve {
                    warn!(
                        "⚠️ Low balance for {:?} on chain {}: {} (min required: {})",
                        token, chain_id, balance, min_reserve
                    );
                }
            }
        }

        Ok(())
    }

    async fn record_error(&self, error: String) {
        let mut metrics = self.metrics.write().await;
        metrics.last_error = Some(error);
    }

    pub async fn get_metrics(&self) -> SolverMetrics {
        self.metrics.read().await.clone()
    }
}
