use std::{env, sync::Arc};

use anyhow::{Context, Result, anyhow};
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, U256},
};
use tracing::{debug, error, info, warn};

use crate::{
    database::database::Database,
    models::{model::IntentCreatedEvent, traits::ChainRelayer},
    relay_coordinator::model::{EthereumConfig, EthereumRelayer},
};

pub mod ethereum_contracts {
    use super::*;

    abigen!(
        EthIntentPool,
        r#"[
            function createIntent(bytes32 intentId, bytes32 commitment, address sourceToken, uint256 sourceAmount, address destToken, uint256 destAmount, uint32 destChain, address refundTo, uint64 customDeadline) external payable
            function settleIntent(bytes32 intentId, address solver, bytes32[] calldata merkleProof, uint256 leafIndex) external
            function syncDestChainRoot(uint32 chainId, bytes32 root) external
            function syncDestChainFillRoot(uint32 chainId, bytes32 root) external
            function refund(bytes32 intentId) external
            function getMerkleRoot() external view returns (bytes32)
            function getDestChainRoot(uint32 chainId) external view returns (bytes32)
            function destChainFillRoots(uint32 chainId) external view returns (bytes32)
            function getIntent(bytes32 intentId) external view returns (tuple(bytes32 commitment, address sourceToken, uint256 sourceAmount, address destToken, uint256 destAmount, uint32 destChain, uint64 deadline, address refundTo, bool filled, bool refunded))
        ]"#
    );

    abigen!(
        EthSettlement,
        r#"[
       function registerIntent(bytes32 intentId, bytes32 commitment, address token, uint256 amount, uint32 sourceChain, uint64 deadline, bytes32 sourceRoot, bytes32[] calldata proof, uint256 leafIndex) external
        function fillIntent(bytes32 intentId, bytes32 commitment, uint32 sourceChain, address token, uint256 amount) external payable
        function claimWithdrawal(bytes32 intentId, bytes32 nullifier, address recipient, bytes32 secret, bytes calldata claimAuth) external
        function syncSourceChainCommitmentRoot(uint32 chainId, bytes32 root) external
        function getMerkleRoot() external view returns (bytes32)
        function generateFillProof(bytes32 intentId) external view returns (bytes32[] memory)
        function getFillTreeSize() external view returns (uint256)
        function getFillIndex(bytes32 intentId) external view returns (uint256)
        function getFill(bytes32 intentId) external view returns (tuple(address solver, address token, uint256 amount, uint32 sourceChain, uint32 timestamp, bool claimed))
        function getSourceChainRoot(uint32 chainId) external view returns (bytes32)
        function sourceChainCommitmentRoots(uint32 chainId) external view returns (bytes32)
        function getIntentParams(bytes32 intentId) external view returns (tuple(bytes32 commitment, address token, uint256 amount, uint32 sourceChain, uint64 deadline, bool exists))
    ]"#
    );
}

use ethereum_contracts::{EthIntentPool, EthSettlement};

pub type EthClient = SignerMiddleware<Provider<Http>, LocalWallet>;

const MANTLE_CHAIN_ID: u32 = 5003;
const TX_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

impl EthereumRelayer {
    pub async fn new(config: EthereumConfig, database: Arc<Database>) -> Result<Self> {
        config.validate()?;
        info!("🔗 Initializing Ethereum relayer");

        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .context("Failed to create Ethereum provider")?
            .interval(std::time::Duration::from_millis(2000));

        let chain_id = provider
            .get_chainid()
            .await
            .context("Failed to get Ethereum chain ID")?
            .as_u64();

        let wallet: LocalWallet = config
            .private_key
            .parse::<LocalWallet>()
            .context("Invalid Ethereum private key")?
            .with_chain_id(chain_id);

        let client = Arc::new(SignerMiddleware::new(provider, wallet));

        let intent_pool_address: Address = config
            .intent_pool_address
            .parse()
            .context("Invalid Ethereum intent pool address")?;

        let settlement_address: Address = config
            .settlement_address
            .parse()
            .context("Invalid Ethereum settlement address")?;

        let intent_pool = EthIntentPool::new(intent_pool_address, client.clone());
        let settlement = EthSettlement::new(settlement_address, client.clone());

        info!("   IntentPool: {:?}", intent_pool_address);
        info!("   Settlement: {:?}", settlement_address);

        Ok(Self {
            client,
            intent_pool,
            settlement,
            database,
            chain_id: chain_id as u32,
        })
    }

    pub async fn health_check(&self) -> Result<()> {
        self.client
            .get_block_number()
            .await
            .context("Ethereum RPC health check failed")?;
        Ok(())
    }

    pub async fn settle_intent(
        &self,
        intent_id: &str,
        solver_address: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> Result<String> {
        let start = std::time::Instant::now();
        info!(
            "✅ [Ethereum] Settling intent {} (leaf_index: {})",
            &intent_id[..10],
            leaf_index
        );

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .context("Invalid intent_id hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let proof: Vec<[u8; 32]> = merkle_path
            .iter()
            .map(|p| {
                hex::decode(&p[2..])
                    .context("Invalid proof hex")
                    .and_then(|decoded| {
                        decoded
                            .try_into()
                            .map_err(|_| anyhow!("Invalid proof element length"))
                    })
            })
            .collect::<Result<Vec<[u8; 32]>>>()?;

        let solver_addr: Address = solver_address.parse().context("Invalid solver address")?;

        let tx = self.intent_pool.settle_intent(
            intent_id_bytes,
            solver_addr,
            proof,
            U256::from(leaf_index),
        );

        match tx.call().await {
            Ok(_) => debug!("   ✓ Transaction simulation successful"),
            Err(e) => {
                let revert_reason = Self::extract_revert_reason(&e);
                error!(
                    "💥 [Ethereum] Settle intent would revert: {}",
                    revert_reason
                );
                return Err(anyhow!("Settlement simulation failed: {}", revert_reason));
            }
        }

        let pending = tx
            .send()
            .await
            .context("Failed to send settle transaction")?;
        let tx_hash = format!("{:?}", pending.tx_hash());

        self.log_transaction(intent_id, "settle_intent", &tx_hash, "pending")
            .await?;

        let receipt = tokio::time::timeout(TX_TIMEOUT, pending)
            .await
            .context("Transaction timed out")?
            .context("Transaction failed")?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };
        self.log_transaction(intent_id, "settle_intent", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Settlement transaction reverted"));
        }

        info!("   ✅ Settled ({}ms)", start.elapsed().as_millis());
        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn execute_refund(&self, intent_id: &str) -> Result<String> {
        let start = std::time::Instant::now();
        info!("♻️ [Ethereum] Refunding intent {}", &intent_id[..10]);

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .context("Invalid intent_id hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let (
            _commitment,
            _source_token,
            _source_amount,
            _dest_token,
            _dest_amount,
            _dest_chain,
            deadline,
            _refund_to,
            filled,
            refunded,
        ) = self.intent_pool.get_intent(intent_id_bytes).call().await?;

        if filled {
            return Err(anyhow!("Intent already filled, cannot refund"));
        }

        if refunded {
            return Err(anyhow!("Intent already refunded"));
        }

        if deadline
            > std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
        {
            return Err(anyhow!("Intent not expired yet, deadline: {}", deadline));
        }

        // Simulate the transaction
        let tx = self.intent_pool.refund(intent_id_bytes);

        if let Err(e) = tx.call().await {
            let revert_reason = Self::extract_revert_reason(&e);
            error!("💥 [Ethereum] Refund would revert: {}", revert_reason);
            return Err(anyhow!("Refund simulation failed: {}", revert_reason));
        }

        // Send the transaction
        let pending = tx.send().await.context("Failed to send refund tx")?;
        let tx_hash = format!("{:?}", pending.tx_hash());
        info!("   📤 Tx sent: {}", &tx_hash[..10]);

        self.log_transaction(intent_id, "refund_intent", &tx_hash, "pending")
            .await?;

        // Wait for confirmation
        let receipt = tokio::time::timeout(TX_TIMEOUT, pending)
            .await
            .context("Refund tx timed out")?
            .context("Refund tx failed")?
            .ok_or_else(|| anyhow!("Refund tx dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };

        self.log_transaction(intent_id, "refund_intent", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Refund transaction reverted"));
        }

        info!("   ✅ Refunded ({}ms)", start.elapsed().as_millis());
        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn register_intent(
        &self,
        intent_id: &str,
        commitment: &str,
        token: &str,
        amount: &str,
        source_chain: u32,
        deadline: u64,
        source_root: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> Result<String> {
        let start = std::time::Instant::now();
        info!(
            "📝 [Ethereum] Registering intent {} (leaf_index: {})",
            &intent_id[..10],
            leaf_index
        );

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .context("Invalid intent_id hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let commitment_bytes: [u8; 32] = hex::decode(&commitment[2..])
            .context("Invalid commitment hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid commitment length"))?;

        let token_address: Address = token.parse().context("Invalid token address")?;
        let amount_u256 = U256::from_dec_str(amount).context("Invalid amount")?;

        let source_root_bytes: [u8; 32] = hex::decode(&source_root[2..])
            .context("Invalid root hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid root length"))?;

        let proof: Vec<[u8; 32]> = merkle_path
            .iter()
            .map(|p| {
                hex::decode(&p[2..])
                    .context("Failed to decode proof element")
                    .and_then(|decoded| {
                        decoded
                            .try_into()
                            .map_err(|_| anyhow!("Invalid proof length"))
                    })
            })
            .collect::<Result<Vec<[u8; 32]>>>()?;

        let tx = self.settlement.register_intent(
            intent_id_bytes,
            commitment_bytes,
            token_address,
            amount_u256,
            source_chain,
            deadline,
            source_root_bytes,
            proof,
            U256::from(leaf_index),
        );

        match tx.call().await {
            Ok(_) => info!("   ✓ Simulation successful"),
            Err(e) => {
                let revert_reason = Self::extract_revert_reason(&e);
                error!(
                    "💥 [Ethereum] Register intent would revert: {}",
                    revert_reason
                );
                return Err(anyhow!("Registration simulation failed: {}", revert_reason));
            }
        }

        let pending = tx
            .send()
            .await
            .context("Failed to send register intent tx")?;
        let tx_hash = format!("{:?}", pending.tx_hash());

        self.log_transaction(intent_id, "register_intent", &tx_hash, "pending")
            .await?;

        let receipt = tokio::time::timeout(TX_TIMEOUT, pending)
            .await
            .context("Registration tx timed out")?
            .context("Registration tx failed")?
            .ok_or_else(|| anyhow!("Registration tx dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };
        self.log_transaction(intent_id, "register_intent", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Registration transaction reverted on-chain"));
        }

        info!("   ✅ Registered ({}ms)", start.elapsed().as_millis());
        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn claim_withdrawal(
        &self,
        intent_id: &str,
        nullifier: &str,
        recipient: &str,
        secret: &str,
        claim_auth: &[u8],
    ) -> Result<String> {
        let start = std::time::Instant::now();
        info!("🔓 [Ethereum] Claiming withdrawal {}", &intent_id[..10]);

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .context("Invalid intent_id hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let nullifier_bytes: [u8; 32] = hex::decode(&nullifier[2..])
            .context("Invalid nullifier hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid nullifier length"))?;

        let recipient_address: Address = recipient.parse().context("Invalid recipient address")?;

        let secret_bytes: [u8; 32] = hex::decode(&secret[2..])
            .context("Invalid secret hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid secret length"))?;

        let tx = self.settlement.claim_withdrawal(
            intent_id_bytes,
            nullifier_bytes,
            recipient_address,
            secret_bytes,
            Bytes::from(claim_auth.to_vec()),
        );

        if let Err(e) = tx.call().await {
            let revert_reason = Self::extract_revert_reason(&e);
            error!("💥 [Ethereum] Claim would revert: {}", revert_reason);
            return Err(anyhow!("Claim simulation failed: {}", revert_reason));
        }

        let pending = tx.send().await.context("Failed to send claim tx")?;
        let tx_hash = format!("{:?}", pending.tx_hash());

        self.log_transaction(intent_id, "claim_withdrawal", &tx_hash, "pending")
            .await?;

        let receipt = tokio::time::timeout(TX_TIMEOUT, pending)
            .await
            .context("Claim tx timed out")?
            .context("Claim tx failed")?
            .ok_or_else(|| anyhow!("Claim tx dropped"))?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Claim transaction reverted"));
        }

        info!("   ✅ Claimed ({}ms)", start.elapsed().as_millis());
        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn get_intent_pool_root(&self) -> Result<String> {
        let root = self.intent_pool.get_merkle_root().call().await?;
        Ok(format!("0x{}", hex::encode(root)))
    }

    pub async fn get_synced_mantle_commitment_root(&self) -> Result<String> {
        let root_bytes: [u8; 32] = self
            .settlement
            .source_chain_commitment_roots(MANTLE_CHAIN_ID)
            .call()
            .await
            .context("Failed to read Mantle commitment root")?;

        Ok(format!("0x{}", hex::encode(root_bytes)))
    }

    pub async fn sync_source_chain_commitment_root_tx(
        &self,
        chain_id: u32,
        root: [u8; 32],
    ) -> Result<String> {
        let start = std::time::Instant::now();
        info!(
            "🌳 [Ethereum] Syncing source chain {} commitment root",
            chain_id
        );

        self.check_balance().await?;

        let tx = self
            .settlement
            .sync_source_chain_commitment_root(chain_id, root);

        if let Err(e) = tx.call().await {
            let revert_reason = Self::extract_revert_reason(&e);
            error!("💥 [Ethereum] Root sync would revert: {}", revert_reason);
            return Err(anyhow!("Root sync simulation failed: {}", revert_reason));
        }

        let pending = tx.send().await.context("Failed to send sync tx")?;
        let receipt = tokio::time::timeout(TX_TIMEOUT, pending)
            .await
            .context("Sync tx timed out")?
            .context("Sync tx failed")?
            .ok_or_else(|| anyhow!("Sync tx dropped"))?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Root sync transaction reverted"));
        }

        info!("   ✅ Root synced ({}ms)", start.elapsed().as_millis());
        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn sync_dest_chain_fill_root_tx(
        &self,
        chain_id: u32,
        root: [u8; 32],
    ) -> Result<String> {
        let start = std::time::Instant::now();
        info!("🌳 [Ethereum] Syncing dest chain {} fill root", chain_id);

        self.check_balance().await?;

        let tx = self.intent_pool.sync_dest_chain_fill_root(chain_id, root);

        if let Err(e) = tx.call().await {
            let revert_reason = Self::extract_revert_reason(&e);
            error!(
                "💥 [Ethereum] Fill root sync would revert: {}",
                revert_reason
            );
            return Err(anyhow!(
                "Fill root sync simulation failed: {}",
                revert_reason
            ));
        }

        let pending = tx
            .send()
            .await
            .context("Failed to send fill root sync tx")?;
        let receipt = tokio::time::timeout(TX_TIMEOUT, pending)
            .await
            .context("Fill root sync tx timed out")?
            .context("Fill root sync tx failed")?
            .ok_or_else(|| anyhow!("Fill root sync tx dropped"))?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Fill root sync transaction reverted"));
        }

        info!("   ✅ Fill root synced ({}ms)", start.elapsed().as_millis());
        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn get_synced_mantle_fill_root(&self) -> Result<String> {
        let root_bytes: [u8; 32] = self
            .intent_pool
            .dest_chain_fill_roots(MANTLE_CHAIN_ID)
            .call()
            .await
            .context("Failed to read Mantle fill root from Ethereum IntentPool")?;

        Ok(format!("0x{}", hex::encode(root_bytes)))
    }

    pub async fn fetch_all_intent_created_events(
        &self,
        from_block: u64,
    ) -> Result<Vec<IntentCreatedEvent>> {
        use ethers::types::{Filter, H256};

        const BATCH_SIZE: u64 = 2000;
        const DELAY_MS: u64 = 300;

        let rpc_url = env::var("ETHEREUM_RPC_URL").context("ETHEREUM_RPC_URL not set")?;
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| anyhow!("Failed to create provider: {}", e))?;

        let current_block = provider
            .get_block_number()
            .await
            .context("Failed to get current block number")?
            .as_u64();

        info!(
            "📦 [Ethereum] Fetching events from block {} to {}",
            from_block, current_block
        );

        let event_signature = ethers::core::utils::keccak256(
            "IntentCreated(bytes32,bytes32,uint32,address,uint256,address,uint256)",
        );
        let topic = H256::from_slice(&event_signature);

        let mut all_events = Vec::new();
        let mut start = from_block;

        while start <= current_block {
            let end = std::cmp::min(start + BATCH_SIZE - 1, current_block);

            let filter = Filter::new()
                .address(self.intent_pool.address())
                .from_block(start)
                .to_block(end)
                .topic0(topic);

            match provider.get_logs(&filter).await {
                Ok(logs) => {
                    debug!(
                        "    ✅ Found {} events in batch {}-{}",
                        logs.len(),
                        start,
                        end
                    );

                    for log in logs {
                        // Skip malformed logs
                        if log.topics.len() < 3 || log.data.len() < 160 {
                            continue;
                        }

                        let intent_id = format!("0x{}", hex::encode(&log.topics[1]));
                        let commitment = format!("0x{}", hex::encode(&log.topics[2]));

                        let data = &log.data;
                        let dest_chain =
                            u32::from_be_bytes([data[28], data[29], data[30], data[31]]);
                        let source_token = Address::from_slice(&data[44..64]);
                        let source_amount = U256::from_big_endian(&data[64..96]);
                        let dest_token = Address::from_slice(&data[108..128]);
                        let dest_amount = U256::from_big_endian(&data[128..160]);

                        all_events.push(IntentCreatedEvent {
                            intent_id,
                            commitment,
                            source_token: format!("{:?}", source_token),
                            source_amount: source_amount.to_string(),
                            dest_token: format!("{:?}", dest_token),
                            dest_amount: dest_amount.to_string(),
                            dest_chain,
                            deadline: None,
                            block_number: log.block_number.map(|b| b.as_u64()),
                            transaction_hash: log.transaction_hash.map(|h| format!("{:?}", h)),
                            log_index: log.log_index.map(|i| i.as_u64()),
                        });
                    }
                }
                Err(e) => {
                    warn!("    ⚠️  Batch failed: {}. Retrying...", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue; // Retry logic
                }
            }

            start = end + 1;
            if start <= current_block {
                tokio::time::sleep(tokio::time::Duration::from_millis(DELAY_MS)).await;
            }
        }

        info!("✅ [Ethereum] Total events fetched: {}", all_events.len());
        Ok(all_events)
    }

    pub async fn check_balance(&self) -> Result<U256> {
        let address = self.client.signer().address();
        let balance = self
            .client
            .get_balance(address, None)
            .await
            .context("Failed to get balance")?;

        debug!(
            "💰 Ethereum balance: {} ETH",
            ethers::utils::format_ether(balance)
        );

        if balance < ethers::utils::parse_ether("0.1")? {
            warn!("⚠️  Low ETH balance! Please fund: {:?}", address);
        }
        Ok(balance)
    }

    fn extract_revert_reason<E: std::fmt::Display>(error: &E) -> String {
        let error_str = error.to_string();
        if error_str.contains("execution reverted:") {
            if let Some(start) = error_str.find("execution reverted:") {
                return error_str[start + 19..]
                    .split('\n')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
            }
        }
        error_str
    }

    async fn log_transaction(
        &self,
        intent_id: &str,
        tx_type: &str,
        tx_hash: &str,
        status: &str,
    ) -> Result<()> {
        self.database
            .log_chain_transaction(intent_id, self.chain_id, tx_type, tx_hash, status)
            .context("Failed to log transaction")
    }

    pub async fn get_fill_proof(&self, intent_id: &str) -> Result<Vec<String>> {
        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let proof = self
            .settlement
            .generate_fill_proof(intent_id_bytes)
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get fill proof: {}", e))?;

        Ok(proof
            .iter()
            .map(|p| format!("0x{}", hex::encode(p)))
            .collect())
    }

    pub async fn get_fill_index(&self, intent_id: &str) -> Result<u32> {
        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let index = self
            .settlement
            .get_fill_index(intent_id_bytes)
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get fill index: {}", e))?;

        Ok(index.as_u32())
    }

    pub async fn check_intent_registered(&self, intent_id: &str) -> Result<bool> {
        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .context("Invalid intent_id")?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let (_, _, _, _, _, exists) = self
            .settlement
            .get_intent_params(intent_id_bytes)
            .call()
            .await?;

        Ok(exists)
    }

    pub async fn check_intent_filled(&self, intent_id: &str) -> Result<bool> {
        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let fill_data = self.settlement.get_fill(intent_id_bytes).call().await?;

        let solver = fill_data.0;
        let is_filled = solver != ethers::types::Address::zero();

        debug!(
            "🔍 [Ethereum] check_intent_filled({}): solver={:?}, is_filled={}",
            &intent_id[..10],
            solver,
            is_filled
        );

        Ok(is_filled)
    }

    pub async fn get_fill_root(&self) -> Result<String> {
        let root = self
            .settlement
            .get_merkle_root()
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get fill merkle root: {}", e))?;

        Ok(format!("0x{}", hex::encode(root)))
    }
}

impl ChainRelayer for EthereumRelayer {
    async fn get_merkle_root(&self) -> Result<String> {
        self.database
            .get_latest_root("ethereum_commitments")?
            .ok_or_else(|| anyhow!("No Ethereum root found"))
    }

    fn sync_source_chain_root(
        &self,
        chain_id: u32,
        root: [u8; 32],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move {
            self.sync_source_chain_commitment_root_tx(chain_id, root)
                .await
        }
    }

    fn sync_dest_chain_root(
        &self,
        chain_id: u32,
        root: [u8; 32],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { self.sync_dest_chain_fill_root_tx(chain_id, root).await }
    }

    fn claim_withdrawal(
        &self,
        intent_id: &str,
        nullifier: &str,
        recipient: &str,
        secret: &str,
        claim_auth: &[u8],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let (id, null, recp, sec, auth) = (
            intent_id.to_string(),
            nullifier.to_string(),
            recipient.to_string(),
            secret.to_string(),
            claim_auth.to_vec(),
        );
        async move { self.claim_withdrawal(&id, &null, &recp, &sec, &auth).await }
    }

    fn mark_filled(
        &self,
        intent_id: &str,
        solver: &str,
        proof: &[String],
        index: u32,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let (id, sol, prf) = (intent_id.to_string(), solver.to_string(), proof.to_vec());
        async move { self.settle_intent(&id, &sol, &prf, index).await }
    }

    fn refund_intent(
        &self,
        intent_id: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let id = intent_id.to_string();
        async move { self.execute_refund(&id).await }
    }
}
