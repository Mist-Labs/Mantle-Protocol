use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use crate::{
    model::{
        ActiveFill, DetectedIntent, FillOpportunity, FillStatus, SolverConfig, SolverMetrics,
        SupportedToken,
    },
    pricefeed::PriceFeedManager,
};
use anyhow::{Context, Result, anyhow};
use ethers::{
    contract::abigen,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{LocalWallet, Signer, Wallet},
    types::{Address, Filter, H256, Log, U256},
    utils::hex,
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
            function getFillIndex(bytes32 intentId) external view returns (uint256)
            event IntentRegistered(bytes32 indexed intentId, bytes32 commitment, address destToken, uint256 destAmount, uint32 sourceChain, uint64 deadline, bytes32[] proof, uint256 leafIndex)
            event IntentFilled(bytes32 indexed intentId, address indexed solver, address indexed token, uint256 amount)
            event WithdrawalClaimed(bytes32 indexed intentId, bytes32 indexed nullifier, address token)
    ]"#
);

abigen!(
    IntentPoolContract,
    r#"[
        function generateCommitmentProof(bytes32 commitment) external view returns (bytes32[] memory, uint256)
        function settleIntent(bytes32 intentId, address solver, bytes32[] calldata merkleProof, uint256 leafIndex) external
        event IntentCreated(bytes32 indexed intentId, bytes32 indexed commitment, uint32 destChain, address sourceToken, uint256 sourceAmount, address destToken, uint256 destAmount)
        event IntentSettled(bytes32 indexed intentId, address indexed solver, bytes32 fillRoot)
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
    pub fn symbol(&self) -> &str {
        match self {
            Self::ETH => "ETH",
            Self::WETH => "WETH",
            Self::USDC => "USDC",
            Self::USDT => "USDT",
            Self::MNT => "MNT",
        }
    }

    pub fn address(&self, chain_id: u64) -> Address {
        match (self, chain_id) {
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
            Self::ETH | Self::WETH | Self::MNT => U256::from(10).pow(U256::from(15)),
            Self::USDC | Self::USDT => U256::from(10).pow(U256::from(6)),
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
    ethereum_settlement:
        SettlementContract<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,
    mantle_settlement: SettlementContract<SignerMiddleware<Arc<Provider<Ws>>, Wallet<SigningKey>>>,
    active_fills: Arc<RwLock<HashMap<H256, ActiveFill>>>,
    processed_intents: Arc<RwLock<HashMap<H256, bool>>>,
    metrics: Arc<RwLock<SolverMetrics>>,
    token_balances: Arc<RwLock<HashMap<(SupportedToken, u64), U256>>>,
    price_feed: Arc<PriceFeedManager>,
}

impl CrossChainSolver {
    pub async fn new(config: SolverConfig, price_feed: Arc<PriceFeedManager>) -> Result<Self> {
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
            active_fills: Arc::new(RwLock::new(HashMap::new())),
            processed_intents: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(SolverMetrics::default())),
            token_balances: Arc::new(RwLock::new(HashMap::new())),
            price_feed,
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

    async fn monitor_ethereum_registered_intents(self: Arc<Self>) -> Result<()> {
        info!("👀 Monitoring Ethereum Settlement IntentRegistered events");

        let filter = Filter::new()
            .address(self.config.ethereum_settlement)
            .event(
                "IntentRegistered(bytes32,bytes32,address,uint256,uint32,uint64,bytes32[],uint256)",
            );
        let mut last_block = self.ethereum_provider.get_block_number().await?.as_u64();
        let mut poll_interval = interval(Duration::from_secs(12));

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
        info!("👀 Monitoring Mantle Settlement IntentRegistered events");

        let filter = Filter::new().address(self.config.mantle_settlement).event(
            "IntentRegistered(bytes32,bytes32,address,uint256,uint32,uint64,bytes32[],uint256)",
        );
        let mut last_block = self.mantle_provider.get_block_number().await?.as_u64();
        let mut poll_interval = interval(Duration::from_secs(3));

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

    async fn handle_registered_intent(&self, log: Log, chain_where_detected: u32) -> Result<()> {
        let settlement = if chain_where_detected == self.config.ethereum_chain_id as u32 {
            &self.ethereum_settlement
        } else {
            &self.mantle_settlement
        };

        let event = settlement
            .decode_event::<IntentRegisteredFilter>(
                "IntentRegistered",
                log.topics.clone(),
                log.data.clone(),
            )
            .context("Failed to decode IntentRegistered event")?;

        let intent_id = H256::from(event.intent_id);

        // Immediate check-and-insert to prevent concurrent processing
        {
            let mut processed = self.processed_intents.write().await;
            if processed.contains_key(&intent_id) {
                debug!(
                    "⏭️ Intent {:?} is already processed or cooling down",
                    intent_id
                );
                return Ok(());
            }
            processed.insert(intent_id, true);
        }

        // Execute the actual filling logic
        match self
            .process_intent_logic(log, event, chain_where_detected)
            .await
        {
            Ok(_) => {
                info!("✅ Successfully processed intent {:?}", intent_id);
                Ok(())
            }
            Err(e) => {
                warn!(
                    "❌ Intent {:?} failed: {}. Clearing lock for retry in 12s...",
                    intent_id, e
                );

                // Unlock the intent after 12 seconds to allow the solver to try again
                let processed_cache = self.processed_intents.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(12)).await;
                    let mut processed = processed_cache.write().await;
                    processed.remove(&intent_id);
                    debug!("♻️ Intent {:?} lock released for retries", intent_id);
                });

                Err(e)
            }
        }
    }

    async fn process_intent_logic(
        &self,
        log: Log,
        event: IntentRegisteredFilter,
        chain_where_detected: u32,
    ) -> Result<()> {
        let intent = DetectedIntent {
            intent_id: H256::from(event.intent_id),
            commitment: H256::from(event.commitment),
            token: event.dest_token,
            token_type: self.identify_token(event.dest_token, chain_where_detected as u64)?,
            amount: event.dest_amount,
            source_chain: event.source_chain,
            dest_chain: chain_where_detected,
            source_block: log.block_number.context("Missing block number")?.as_u64(),
            detected_at: chrono::Utc::now().timestamp() as u64,
        };

        let now = chrono::Utc::now().timestamp() as u64;
        if event.deadline <= now {
            return Err(anyhow!("Intent expired"));
        }

        let provider = if chain_where_detected == self.config.ethereum_chain_id as u32 {
            &self.ethereum_provider
        } else {
            &self.mantle_provider
        };

        // Confirmation Wait Loop
        let required_confirmations = 2;
        let mut attempts = 0;
        loop {
            let current_block = provider.get_block_number().await?.as_u64();
            let confirmations = current_block.saturating_sub(intent.source_block);

            if confirmations >= required_confirmations {
                break;
            }
            if attempts >= 60 {
                return Err(anyhow!("Confirmation timeout"));
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
            attempts += 1;
        }

        // On-chain verification
        let settlement = if chain_where_detected == self.config.ethereum_chain_id as u32 {
            &self.ethereum_settlement
        } else {
            &self.mantle_settlement
        };

        let (_, token_check, amount_check, _, _, exists) = settlement
            .get_intent_params(intent.intent_id.0)
            .call()
            .await?;

        if !exists || token_check != intent.token || amount_check != intent.amount {
            return Err(anyhow!("On-chain verification failed or mismatch"));
        }

        let opportunity = self.evaluate_fill_opportunity(&intent).await?;
        if self.should_fill(&opportunity).await? {
            if chain_where_detected == self.config.mantle_chain_id as u32 {
                self.execute_fill_on_mantle(&intent, &opportunity).await?;
            } else {
                self.execute_fill_on_ethereum(&intent, &opportunity).await?;
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

        self.verify_provider_health(self.config.ethereum_chain_id)
            .await
            .context("Provider health check failed")?;

        let (
            _commitment_check,
            _token_check,
            _amount_check,
            _source_chain_check,
            _deadline_check,
            exists,
        ) = self
            .ethereum_settlement
            .get_intent_params(intent.intent_id.0)
            .call()
            .await
            .context("Failed to verify intent before fill")?;

        if !exists {
            return Err(anyhow!(
                "Intent no longer exists on-chain. May have been filled by another solver."
            ));
        }

        let (solver_check, _token, _amount, _source_chain, _timestamp, _claimed) = self
            .ethereum_settlement
            .get_fill(intent.intent_id.0)
            .call()
            .await
            .context("Failed to check fill status")?;

        if solver_check != Address::zero() {
            warn!("⚠️ Intent already filled by solver: {:?}", solver_check);
            return Err(anyhow!("Intent already filled"));
        }

        info!("🔍 Pre-flight balance check...");
        let current_balance = self
            .fetch_balance_inner(intent.token_type, self.config.ethereum_chain_id)
            .await
            .context("Failed to fetch balance for pre-flight check")?;

        let buffer_percent = U256::from(108);
        let required_with_buffer = intent
            .amount
            .saturating_mul(buffer_percent)
            .checked_div(U256::from(100))
            .unwrap_or(intent.amount);

        if current_balance < required_with_buffer {
            return Err(anyhow!(
                "❌ Pre-flight balance check failed: has {} but needs {} (amount: {} + 8% buffer)",
                current_balance,
                required_with_buffer,
                intent.amount
            ));
        }

        info!(
            "✅ Pre-flight balance OK: {} >= {} needed",
            current_balance, required_with_buffer
        );

        let intent_id_bytes: [u8; 32] = intent.intent_id.0;
        let commitment_bytes: [u8; 32] = intent.commitment.0;

        if !intent.token_type.is_native() {
            info!("🔓 Approving ERC20 token...");
            self.approve_token_if_needed(
                intent.token,
                self.config.ethereum_settlement,
                intent.amount,
                self.ethereum_client.clone(),
            )
            .await?;
        }

        info!("📝 Building fill transaction:");
        info!("   Intent ID: 0x{}", hex::encode(intent_id_bytes));
        info!("   Commitment: 0x{}", hex::encode(commitment_bytes));
        info!("   Source chain: {}", intent.source_chain);
        info!("   Token: {:?}", intent.token);
        info!("   Amount: {}", intent.amount);

        let mut tx = self.ethereum_settlement.fill_intent(
            intent_id_bytes,
            commitment_bytes,
            intent.source_chain,
            intent.token,
            intent.amount,
        );

        if intent.token_type.is_native() {
            info!(
                "💰 Sending {} ETH with transaction",
                ethers::utils::format_ether(intent.amount)
            );
            tx = tx.value(intent.amount);
        }

        info!("⛽ Estimating gas...");
        let gas_estimate = match tx.estimate_gas().await {
            Ok(gas) => {
                info!("✅ Gas estimated: {} units", gas);
                gas
            }
            Err(e) => {
                error!("❌ Gas estimation failed: {:?}", e);
                let error_msg = format!("{:?}", e);

                if error_msg.contains("0x2c5211c6") {
                    error!("   Revert reason: IntentNotRegistered()");
                } else if error_msg.contains("0xfb8f41b2") {
                    error!("   Revert reason: InsufficientBalance()");
                    if let Ok(bal) = self
                        .fetch_balance_inner(intent.token_type, self.config.ethereum_chain_id)
                        .await
                    {
                        error!("   Current balance: {}", bal);
                        error!("   Required: {}", intent.amount);
                    }
                }

                return Err(anyhow!("Gas estimation failed: {}", e));
            }
        };

        let gas_with_buffer = gas_estimate.saturating_mul(U256::from(120)) / U256::from(100);
        let tx = tx.gas(gas_with_buffer);

        info!("📤 Sending fill transaction...");
        let pending_tx = tx.send().await.context("Failed to send fill transaction")?;

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
                error!("❌ Fill tx dropped: {:?}", tx_hash);
                let mut active = self.active_fills.write().await;
                if let Some(fill) = active.get_mut(&intent.intent_id) {
                    fill.status = FillStatus::Failed;
                }
                return Err(anyhow!("Transaction dropped"));
            }
        }

        Ok(())
    }

    async fn execute_fill_on_mantle(
        &self,
        intent: &DetectedIntent,
        opportunity: &FillOpportunity,
    ) -> Result<()> {
        info!("🔨 Executing fill on Mantle: {:?}", intent.intent_id);

        self.verify_provider_health(self.config.mantle_chain_id)
            .await
            .context("Mantle provider health check failed")?;

        let (
            _commitment_check,
            _token_check,
            _amount_check,
            _source_chain_check,
            _deadline_check,
            exists,
        ) = self
            .mantle_settlement
            .get_intent_params(intent.intent_id.0)
            .call()
            .await
            .context("Failed to verify intent before fill")?;

        if !exists {
            return Err(anyhow!(
                "Intent no longer exists on-chain. May have been filled by another solver."
            ));
        }

        let (solver_check, _token, _amount, _source_chain, _timestamp, _claimed) = self
            .mantle_settlement
            .get_fill(intent.intent_id.0)
            .call()
            .await
            .context("Failed to check fill status")?;

        if solver_check != Address::zero() {
            warn!("⚠️ Intent already filled by solver: {:?}", solver_check);
            return Err(anyhow!("Intent already filled"));
        }

        info!("🔍 Pre-flight balance check...");
        let current_balance = self
            .fetch_balance_inner(intent.token_type, self.config.mantle_chain_id)
            .await
            .context("Failed to fetch balance for pre-flight check")?;

        let buffer_percent = U256::from(108);
        let required_with_buffer = intent
            .amount
            .saturating_mul(buffer_percent)
            .checked_div(U256::from(100))
            .unwrap_or(intent.amount);

        if current_balance < required_with_buffer {
            return Err(anyhow!(
                "❌ Pre-flight balance check failed: has {} but needs {} (amount: {} + 8% buffer)",
                current_balance,
                required_with_buffer,
                intent.amount
            ));
        }

        info!(
            "✅ Pre-flight balance OK: {} >= {} needed",
            current_balance, required_with_buffer
        );

        let intent_id_bytes: [u8; 32] = intent.intent_id.0;
        let commitment_bytes: [u8; 32] = intent.commitment.0;

        if !intent.token_type.is_native() {
            info!("🔓 Approving ERC20 token...");
            self.approve_token_if_needed(
                intent.token,
                self.config.mantle_settlement,
                intent.amount,
                self.mantle_client.clone(),
            )
            .await?;
        }

        info!("📝 Building fill transaction:");
        info!("   Intent ID: 0x{}", hex::encode(intent_id_bytes));
        info!("   Commitment: 0x{}", hex::encode(commitment_bytes));
        info!("   Source chain: {}", intent.source_chain);
        info!("   Token: {:?}", intent.token);
        info!("   Amount: {}", intent.amount);

        let mut tx = self.mantle_settlement.fill_intent(
            intent_id_bytes,
            commitment_bytes,
            intent.source_chain,
            intent.token,
            intent.amount,
        );

        if intent.token_type.is_native() {
            info!(
                "💰 Sending {} MNT with transaction",
                ethers::utils::format_ether(intent.amount)
            );
            tx = tx.value(intent.amount);
        }

        info!("⛽ Estimating gas...");
        let gas_estimate = match tx.estimate_gas().await {
            Ok(gas) => {
                info!("✅ Gas estimated: {} units", gas);
                gas
            }
            Err(e) => {
                error!("❌ Gas estimation failed: {:?}", e);
                let error_msg = format!("{:?}", e);

                if error_msg.contains("0x2c5211c6") {
                    error!("   Revert reason: IntentNotRegistered()");
                } else if error_msg.contains("0xfb8f41b2") {
                    error!("   Revert reason: InsufficientBalance()");
                    if let Ok(bal) = self
                        .fetch_balance_inner(intent.token_type, self.config.mantle_chain_id)
                        .await
                    {
                        error!("   Current balance: {}", bal);
                        error!("   Required: {}", intent.amount);
                    }
                }

                return Err(anyhow!("Gas estimation failed: {}", e));
            }
        };

        let gas_with_buffer = gas_estimate.saturating_mul(U256::from(120)) / U256::from(100);
        let tx = tx.gas(gas_with_buffer);

        info!("📤 Sending fill transaction...");
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
                error!("❌ Fill tx dropped: {:?}", tx_hash);
                let mut active = self.active_fills.write().await;
                if let Some(fill) = active.get_mut(&intent.intent_id) {
                    fill.status = FillStatus::Failed;
                }
                return Err(anyhow!("Transaction dropped"));
            }
        }

        Ok(())
    }

    async fn evaluate_fill_opportunity(&self, intent: &DetectedIntent) -> Result<FillOpportunity> {
        let settlement_fee_bps = 200u128;
        let fee_amount = intent.amount * U256::from(settlement_fee_bps) / U256::from(10000);
        let gas_estimate = self.estimate_fill_gas(intent).await?;

        let fee_value_usd = self
            .get_token_price_usd(intent.token_type, fee_amount)
            .await?;
        let gas_cost_usd = self.get_gas_cost_usd(gas_estimate).await?;
        let intent_value_usd = self
            .get_token_price_usd(intent.token_type, intent.amount)
            .await?;

        let profit_usd = fee_value_usd - gas_cost_usd;

        let estimated_profit = if profit_usd > 0.0 {
            let profit_per_usd = fee_amount.as_u128() as f64 / fee_value_usd;
            U256::from((profit_usd * profit_per_usd) as u128)
        } else {
            U256::zero()
        };

        let profit_bps = if intent_value_usd > 0.0 {
            ((profit_usd / intent_value_usd) * 10000.0).max(0.0) as u16
        } else {
            0
        };

        debug!(
            "💰 Intent: ${:.6} | Fee: ${:.6} | Gas: ${:.6} | Profit: ${:.6} ({} bps)",
            intent_value_usd, fee_value_usd, gas_cost_usd, profit_usd, profit_bps
        );

        let risk_score = self.calculate_risk_score(intent).await?;

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
            U256::from(100_000)
        } else {
            U256::from(150_000)
        };

        let gas_price = if intent.dest_chain == self.config.ethereum_chain_id as u32 {
            self.ethereum_provider.get_gas_price().await?
        } else {
            self.mantle_provider.get_gas_price().await?
        };

        Ok(base_gas * gas_price)
    }

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

        info!("🔍 Fetching fresh balance for fill decision...");
        let balance = self
            .fetch_balance_with_retry(opportunity.intent.token_type, dest_chain, 3)
            .await?;

        {
            let mut balances = self.token_balances.write().await;
            balances.insert((opportunity.intent.token_type, dest_chain), balance);
        }

        let safety_margin = U256::from(105);
        let required_with_margin = opportunity
            .capital_required
            .saturating_mul(safety_margin)
            .checked_div(U256::from(100))
            .unwrap_or(opportunity.capital_required);

        if balance < required_with_margin {
            warn!(
                "❌ Insufficient balance for {:?} on chain {}: has {} but needs {} (with 5% margin)",
                opportunity.intent.token_type, dest_chain, balance, required_with_margin
            );
            return Ok(false);
        }

        let active_fills = self.active_fills.read().await;
        let locked_capital: U256 = active_fills
            .values()
            .filter(|f| {
                f.token_type == opportunity.intent.token_type
                    && f.dest_chain == dest_chain as u32
                    && (f.status == FillStatus::Pending || f.status == FillStatus::Confirmed)
            })
            .map(|f| f.amount)
            .fold(U256::zero(), |acc, amount| acc.saturating_add(amount));

        let available_balance = balance.saturating_sub(locked_capital);

        if available_balance < required_with_margin {
            warn!(
                "❌ Insufficient available balance: total={}, locked={}, available={}, needed={}",
                balance, locked_capital, available_balance, required_with_margin
            );
            return Ok(false);
        }

        info!(
            "✅ Fill approved: profit={}bps, risk={}, balance={}, available={}, needed={}",
            opportunity.profit_bps,
            opportunity.risk_score,
            balance,
            available_balance,
            required_with_margin
        );

        Ok(true)
    }

    async fn verify_provider_health(&self, chain_id: u64) -> Result<()> {
        let provider = if chain_id == self.config.ethereum_chain_id {
            &self.ethereum_provider
        } else {
            &self.mantle_provider
        };

        let chain_name = if chain_id == self.config.ethereum_chain_id {
            "Ethereum"
        } else {
            "Mantle"
        };

        let block = tokio::time::timeout(Duration::from_secs(5), provider.get_block_number())
            .await
            .context(format!("{} provider timeout", chain_name))?
            .context(format!("{} provider error", chain_name))?;

        debug!("✅ {} provider healthy (block: {})", chain_name, block);
        Ok(())
    }

    async fn get_token_price_usd(&self, token_type: SupportedToken, amount: U256) -> Result<f64> {
        let token_decimals = token_type.decimals();
        let amount_decimal = amount.as_u128() as f64 / 10f64.powi(token_decimals as i32);

        let price_per_token = self.price_feed.get_usd_price(token_type).await?;

        let value_usd = amount_decimal * price_per_token;

        debug!(
            "💵 {} amount {} = ${:.6}",
            token_type.symbol(),
            amount_decimal,
            value_usd
        );

        Ok(value_usd)
    }

    async fn get_gas_cost_usd(&self, gas_amount_wei: U256) -> Result<f64> {
        let gas_amount_eth = gas_amount_wei.as_u128() as f64 / 10f64.powi(18);

        let eth_price = self.price_feed.get_usd_price(SupportedToken::ETH).await?;

        let value_usd = gas_amount_eth * eth_price;

        debug!("💵 Gas {} ETH = ${:.6}", gas_amount_eth, value_usd);

        Ok(value_usd)
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
            .context("Failed to check token allowance")?;

        if allowance >= amount {
            debug!("✅ Sufficient allowance already exists: {}", allowance);
            return Ok(());
        }

        info!(
            "🔓 Approving token: current={}, needed={}",
            allowance, amount
        );

        let call = erc20.approve(spender, U256::max_value());

        match call.send().await {
            Ok(pending) => {
                info!("⏳ Approval tx sent, waiting for confirmation...");
                match pending.await {
                    Ok(Some(receipt)) => {
                        if receipt.status == Some(0.into()) {
                            return Err(anyhow!("Approval transaction reverted"));
                        }
                        info!(
                            "✅ Approval confirmed in block {}",
                            receipt.block_number.unwrap()
                        );
                        Ok(())
                    }
                    Ok(None) => {
                        warn!("⚠️ Approval tx dropped from mempool");
                        Err(anyhow!("Approval transaction dropped"))
                    }
                    Err(e) => {
                        error!("❌ Approval confirmation failed: {}", e);
                        Err(anyhow!("Approval failed: {}", e))
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                if error_msg.contains("already known")
                    || error_msg.contains("replacement transaction underpriced")
                {
                    warn!("⚠️ Approval tx already in mempool, proceeding cautiously...");
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let new_allowance = erc20
                        .allowance(self.config.solver_address, spender)
                        .call()
                        .await
                        .context("Failed to re-check allowance after pending tx")?;

                    if new_allowance >= amount {
                        info!("✅ Pending approval tx confirmed, allowance now sufficient");
                        return Ok(());
                    }

                    warn!("⏳ Waiting for pending approval to confirm...");
                    tokio::time::sleep(Duration::from_secs(5)).await;

                    let final_allowance = erc20
                        .allowance(self.config.solver_address, spender)
                        .call()
                        .await
                        .context("Failed final allowance check")?;

                    if final_allowance >= amount {
                        info!("✅ Approval confirmed after wait");
                        Ok(())
                    } else {
                        Err(anyhow!(
                            "Approval still pending after 8s wait, aborting fill"
                        ))
                    }
                } else {
                    error!("❌ Approval tx send failed: {}", e);
                    Err(anyhow!("Approve failed: {}", e))
                }
            }
        }
    }

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

                if let Err(e) = self.process_confirmed_fill(&fill).await {
                    error!("❌ Error processing confirmed fill: {}", e);
                }
            }
        }
    }

    async fn process_confirmed_fill(&self, fill: &ActiveFill) -> Result<()> {
        let required_confirmations = 6;

        let current_block = if fill.dest_chain == self.config.ethereum_chain_id as u32 {
            self.ethereum_provider.get_block_number().await?.as_u64()
        } else {
            self.mantle_provider.get_block_number().await?.as_u64()
        };

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
                "⏳ Waiting for confirmations ({}/{}) for intent: {:?}",
                confirmations, required_confirmations, fill.intent_id
            );
            return Ok(());
        }

        info!(
            "✅ Fill confirmed with {} confirmations. Waiting for relayer to settle...",
            confirmations
        );

        {
            let mut active = self.active_fills.write().await;
            if let Some(f) = active.get_mut(&fill.intent_id) {
                f.status = FillStatus::Claimed;
            }
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.successful_fills += 1;
            metrics.active_fills_count = metrics.active_fills_count.saturating_sub(1);
        }

        Ok(())
    }

    async fn get_token_balance(&self, token: SupportedToken, chain_id: u64) -> Result<U256> {
        let key = (token, chain_id);

        {
            let balances = self.token_balances.read().await;
            if let Some(balance) = balances.get(&key) {
                info!("Balance of {:?}: {}", token, balance);
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
