use std::sync::Arc;

use anyhow::{Result, anyhow};
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, U256},
};
use tracing::{info, warn};

use crate::{
    database::database::Database,
    relay_coordinator::model::{MantleConfig, MantleRelayer},
};

pub mod mantle_contracts {
    use super::*;

    abigen!(
        MantleIntentPool,
        r#"[
            function createIntent(bytes32 intentId, bytes32 commitment, address token, uint256 amount, uint32 destChain, address refundTo, bytes32 secret, bytes32 nullifier) external
            function markFilled(bytes32 intentId, bytes32[] calldata merkleProof, uint256 leafIndex) external
            function syncDestChainRoot(uint32 chainId, bytes32 root) external
            function refund(bytes32 intentId) external
            function generateCommitmentProof(bytes32 commitment) external view returns (bytes32[] memory, uint256)
            function getCommitmentRoot() external view returns (bytes32)
        ]"#
    );

    abigen!(
        MantleSettlement,
        r#"[
            function registerIntent(bytes32 intentId, bytes32 commitment, address token, uint256 amount, uint32 sourceChain, uint64 deadline, bytes32 sourceRoot, bytes32[] calldata proof, uint256 leafIndex) external
            function fillIntent(bytes32 intentId, bytes32 commitment, uint32 sourceChain, address token, uint256 amount) external payable
            function claimWithdrawal(bytes32 intentId, bytes32 nullifier, address recipient, bytes32 secret, bytes calldata claimAuth) external
            function syncSourceChainRoot(uint32 chainId, bytes32 root) external
            function getMerkleRoot() external view returns (bytes32)
            function generateFillProof(bytes32 intentId) external view returns (bytes32[] memory)
            function getFillTreeSize() external view returns (uint256)
            function getFill(bytes32 intentId) external view returns (tuple(address solver, address token, uint256 amount, uint32 sourceChain, uint32 timestamp, bool claimed))
       ]"#
    );
}

use mantle_contracts::{MantleIntentPool, MantleSettlement};

pub type MantleClient = SignerMiddleware<Provider<Http>, LocalWallet>;

impl MantleRelayer {
    pub async fn new(config: MantleConfig, database: Arc<Database>) -> Result<Self> {
        config.validate()?;
        info!("🔗 Initializing Mantle relayer");

        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .map_err(|e| anyhow!("Failed to create provider: {}", e))?;

        let chain_id = provider
            .get_chainid()
            .await
            .map_err(|e| anyhow!("Failed to get chain ID: {}", e))?
            .as_u64();

        let wallet: LocalWallet = config
            .private_key
            .parse::<LocalWallet>()
            .map_err(|e| anyhow!("Invalid private key: {}", e))?
            .with_chain_id(chain_id);

        let client = Arc::new(SignerMiddleware::new(provider, wallet));

        let intent_pool_address: Address = config
            .intent_pool_address
            .parse()
            .map_err(|e| anyhow!("Invalid intent pool address: {}", e))?;

        let settlement_address: Address = config
            .settlement_address
            .parse()
            .map_err(|e| anyhow!("Invalid settlement address: {}", e))?;

        let intent_pool = MantleIntentPool::new(intent_pool_address, client.clone());
        let settlement = MantleSettlement::new(settlement_address, client.clone());

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
            .map_err(|e| anyhow!("Mantle RPC unhealthy: {}", e))?;
        Ok(())
    }

    pub async fn create_intent(
        &self,
        intent_id: &str,
        commitment: &str,
        token: &str,
        amount: &str,
        dest_chain: u32,
        refund_to: &str,
        secret: &str,
        nullifier: &str,
    ) -> Result<String> {
        info!("🔨 Creating intent on Mantle");

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let commitment_bytes: [u8; 32] = hex::decode(&commitment[2..])
            .map_err(|e| anyhow!("Invalid commitment hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid commitment length"))?;

        let token_address: Address = token
            .parse()
            .map_err(|e| anyhow!("Invalid token address: {}", e))?;

        let amount_u256 =
            U256::from_dec_str(amount).map_err(|e| anyhow!("Invalid amount: {}", e))?;

        let refund_to_address: Address = refund_to
            .parse()
            .map_err(|e| anyhow!("Invalid refund address: {}", e))?;

        let secret_bytes: [u8; 32] = hex::decode(&secret[2..])
            .map_err(|e| anyhow!("Invalid secret hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid secret length"))?;

        let nullifier_bytes: [u8; 32] = hex::decode(&nullifier[2..])
            .map_err(|e| anyhow!("Invalid nullifier hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid nullifier length"))?;

        let tx = self.intent_pool.create_intent(
            intent_id_bytes,
            commitment_bytes,
            token_address,
            amount_u256,
            dest_chain,
            refund_to_address,
            secret_bytes,
            nullifier_bytes,
        );

        let pending = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let tx_hash = format!("{:?}", pending.tx_hash());
        info!("📤 Intent creation transaction sent: {}", tx_hash);

        self.log_transaction(intent_id, "create_intent", &tx_hash, "pending")
            .await?;

        let receipt = pending
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };

        self.log_transaction(intent_id, "create_intent", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

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
        info!("📝 Registering intent on Mantle Settlement");

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let commitment_bytes: [u8; 32] = hex::decode(&commitment[2..])
            .map_err(|e| anyhow!("Invalid commitment hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid commitment length"))?;

        let token_address: Address = token
            .parse()
            .map_err(|e| anyhow!("Invalid token address: {}", e))?;

        let amount_u256 =
            U256::from_dec_str(amount).map_err(|e| anyhow!("Invalid amount: {}", e))?;

        let source_root_bytes: [u8; 32] = hex::decode(&source_root[2..])
            .map_err(|e| anyhow!("Invalid root hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid root length"))?;
        let proof: Vec<[u8; 32]> = merkle_path
            .iter()
            .map(|p| {
                let decoded = hex::decode(&p[2..])?;
                let array: [u8; 32] = decoded
                    .try_into()
                    .map_err(|_| anyhow!("Invalid proof length"))?;
                Ok(array)
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

        let pending = tx.send().await?;
        let tx_hash = format!("{:?}", pending.tx_hash());

        info!("📤 Register intent tx sent: {}", tx_hash);

        self.log_transaction(intent_id, "register_intent", &tx_hash, "pending")
            .await?;

        let receipt = pending
            .await?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };
        self.log_transaction(intent_id, "register_intent", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn execute_fill_intent(
        &self,
        intent_id: &str,
        commitment: &str,
        source_chain: u32,
        token: &str,
        amount: &str,
    ) -> Result<String> {
        info!("🔨 Filling intent on Mantle (solver action)");

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let commitment_bytes: [u8; 32] = hex::decode(&commitment[2..])
            .map_err(|e| anyhow!("Invalid commitment hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid commitment length"))?;

        let token_address: Address = token
            .parse()
            .map_err(|e| anyhow!("Invalid token address: {}", e))?;

        let amount_u256 =
            U256::from_dec_str(amount).map_err(|e| anyhow!("Invalid amount: {}", e))?;

        let tx = self.settlement.fill_intent(
            intent_id_bytes,
            commitment_bytes,
            source_chain,
            token_address,
            amount_u256,
        );

        let pending = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let tx_hash = format!("{:?}", pending.tx_hash());
        info!("📤 Fill transaction sent: {}", tx_hash);

        self.log_transaction(intent_id, "fill_intent", &tx_hash, "pending")
            .await?;

        let receipt = pending
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };

        self.log_transaction(intent_id, "fill_intent", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

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
        info!("🔓 Claiming withdrawal on Mantle");

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let nullifier_bytes: [u8; 32] = hex::decode(&nullifier[2..])
            .map_err(|e| anyhow!("Invalid nullifier hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid nullifier length"))?;

        let recipient_address: Address = recipient
            .parse()
            .map_err(|e| anyhow!("Invalid recipient address: {}", e))?;

        let secret_bytes: [u8; 32] = hex::decode(&secret[2..])
            .map_err(|e| anyhow!("Invalid secret hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid secret length"))?;

        let tx = self.settlement.claim_withdrawal(
            intent_id_bytes,
            nullifier_bytes,
            recipient_address,
            secret_bytes,
            Bytes::from(claim_auth.to_vec()),
        );

        let pending = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let tx_hash = format!("{:?}", pending.tx_hash());
        info!("📤 Claim transaction sent: {}", tx_hash);

        self.log_transaction(intent_id, "claim_withdrawal", &tx_hash, "pending")
            .await?;

        let receipt = pending
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };

        self.log_transaction(intent_id, "claim_withdrawal", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn execute_mark_filled(
        &self,
        intent_id: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> Result<String> {
        info!("✅ Marking intent filled on Mantle");

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let proof: Vec<[u8; 32]> = merkle_path
            .iter()
            .map(|p| {
                let decoded =
                    hex::decode(&p[2..]).map_err(|e| anyhow!("Invalid proof hex: {}", e))?;
                let array: [u8; 32] = decoded
                    .try_into()
                    .map_err(|_| anyhow!("Invalid proof element length"))?;
                Ok(array)
            })
            .collect::<Result<Vec<[u8; 32]>>>()?;

        let tx = self
            .intent_pool
            .mark_filled(intent_id_bytes, proof, U256::from(leaf_index));

        let pending = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let tx_hash = format!("{:?}", pending.tx_hash());
        info!("📤 Mark filled transaction sent: {}", tx_hash);

        self.log_transaction(intent_id, "mark_filled", &tx_hash, "pending")
            .await?;

        let receipt = pending
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };

        self.log_transaction(intent_id, "mark_filled", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn execute_refund(&self, intent_id: &str) -> Result<String> {
        info!("♻️ Refunding intent on Mantle");

        let intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let tx = self.intent_pool.refund(intent_id_bytes);

        let pending = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let tx_hash = format!("{:?}", pending.tx_hash());
        info!("📤 Refund transaction sent: {}", tx_hash);

        self.log_transaction(intent_id, "refund_intent", &tx_hash, "pending")
            .await?;

        let receipt = pending
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        let status = if receipt.status == Some(1.into()) {
            "confirmed"
        } else {
            "reverted"
        };

        self.log_transaction(intent_id, "refund_intent", &tx_hash, status)
            .await?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

        Ok(format!("{:?}", receipt.transaction_hash))
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
            .map_err(|e| anyhow!("Failed to log transaction: {}", e))
    }

    pub async fn get_commitment_proof(&self, commitment: &str) -> Result<(Vec<String>, u32)> {
        let commitment_bytes: [u8; 32] = hex::decode(&commitment[2..])
            .map_err(|e| anyhow!("Invalid commitment hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid commitment length"))?;

        let (proof, leaf_index) = self
            .intent_pool
            .generate_commitment_proof(commitment_bytes)
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get commitment proof: {}", e))?;

        Ok((
            proof
                .iter()
                .map(|p| format!("0x{}", hex::encode(p)))
                .collect(),
            leaf_index.as_u32(),
        ))
    }

    pub async fn get_commitment_root(&self) -> Result<String> {
        let root = self
            .intent_pool
            .get_commitment_root()
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get commitment root from IntentPool: {}", e))?;

        Ok(format!("0x{}", hex::encode(root)))
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

    pub async fn get_fill_root(&self) -> Result<String> {
        let root = self
            .settlement
            .get_merkle_root()
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get merkle root: {}", e))?;

        Ok(format!("0x{}", hex::encode(root)))
    }

    pub async fn get_fill_index(&self, intent_id: &str) -> Result<u32> {
        let _intent_id_bytes: [u8; 32] = hex::decode(&intent_id[2..])
            .map_err(|e| anyhow!("Invalid intent_id hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?;

        let tree_size = self
            .settlement
            .get_fill_tree_size()
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get tree size: {}", e))?;

        Ok((tree_size.as_u64() - 1) as u32)
    }

    pub async fn get_fill_merkle_root(&self) -> Result<String> {
        let root = self
            .settlement
            .get_merkle_root()
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get merkle root: {}", e))?;

        Ok(format!("0x{}", hex::encode(root)))
    }

    pub async fn sync_source_root_tx(&self, chain_id: u32, root: String) -> Result<String> {
        info!("🌳 Syncing source chain {} root on Mantle", chain_id);

        self.check_balance().await?;

        let root_bytes: [u8; 32] = hex::decode(&root[2..])
            .map_err(|e| anyhow!("Invalid root hex: {}", e))?
            .try_into()
            .map_err(|_| anyhow!("Invalid root length"))?;

        let tx = self.settlement.sync_source_chain_root(chain_id, root_bytes);

        let pending = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let receipt = pending
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

        let tx_hash = format!("{:?}", receipt.transaction_hash);
        info!("✅ Source chain root synced: {}", tx_hash);
        Ok(tx_hash)
    }

    pub async fn check_balance(&self) -> Result<U256> {
        let signer = self.client.signer();
        let address = signer.address();

        let balance = self.client.get_balance(address, None).await?;

        info!(
            "💰 Mantle Relayer balance: {} MNT",
            ethers::utils::format_ether(balance)
        );

        if balance < ethers::utils::parse_ether("0.5")? {
            warn!(
                "⚠️  Low MNT balance! Please fund relayer account: {:?}",
                address
            );
        }

        Ok(balance)
    }

    pub async fn sync_dest_root_tx(&self, chain_id: u32, root: [u8; 32]) -> Result<String> {
        info!("🌳 Syncing dest chain {} root on Mantle", chain_id);

        self.check_balance().await?;

        let tx = self.intent_pool.sync_dest_chain_root(chain_id, root);

        let pending = tx
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

        let receipt = pending
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?
            .ok_or_else(|| anyhow!("Transaction dropped"))?;

        if receipt.status != Some(1.into()) {
            return Err(anyhow!("Transaction reverted"));
        }

        let tx_hash = format!("{:?}", receipt.transaction_hash);
        info!("✅ Dest chain root synced: {}", tx_hash);

        Ok(tx_hash)
    }
}

use crate::models::traits::ChainRelayer;

impl ChainRelayer for MantleRelayer {
    fn get_merkle_root(&self) -> impl std::future::Future<Output = Result<String>> + Send {
        async move {
            self.get_fill_merkle_root()
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    fn sync_source_chain_root(
        &self,
        chain_id: u32,
        root: String,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move {
            self.sync_source_root_tx(chain_id, root)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    fn sync_dest_chain_root(
        &self,
        chain_id: u32,
        root: [u8; 32],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move {
            self.sync_dest_root_tx(chain_id, root)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    fn fill_intent(
        &self,
        intent_id: &str,
        commitment: &str,
        source_chain: u32,
        token: &str,
        amount: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let intent_id = intent_id.to_string();
        let commitment = commitment.to_string();
        let token = token.to_string();
        let amount = amount.to_string();

        async move {
            self.execute_fill_intent(&intent_id, &commitment, source_chain, &token, &amount)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    fn claim_withdrawal(
        &self,
        intent_id: &str,
        nullifier: &str,
        recipient: &str,
        secret: &str,
        claim_auth: &[u8],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let intent_id = intent_id.to_string();
        let nullifier = nullifier.to_string();
        let recipient = recipient.to_string();
        let secret = secret.to_string();
        let claim_auth = claim_auth.to_vec();

        async move {
            self.claim_withdrawal(&intent_id, &nullifier, &recipient, &secret, &claim_auth)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    fn mark_filled(
        &self,
        intent_id: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let intent_id = intent_id.to_string();
        let merkle_path = merkle_path.to_vec();

        async move {
            self.execute_mark_filled(&intent_id, &merkle_path, leaf_index)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    fn refund_intent(
        &self,
        intent_id: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let intent_id = intent_id.to_string();

        async move {
            self.execute_refund(&intent_id)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }
}
