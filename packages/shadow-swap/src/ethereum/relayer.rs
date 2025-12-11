use std::sync::Arc;

use anyhow::{Result, anyhow};
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, U256},
};
use tracing::info;

use crate::{
    database::database::Database,
    relay_coordinator::model::{EthereumConfig, EthereumRelayer},
};

pub mod ethereum_contracts {
    use super::*;

    abigen!(
        EthIntentPool,
        r#"[
            function createIntent(bytes32 intentId, bytes32 commitment, address token, uint256 amount, uint32 destChain, address refundTo, bytes32 secret, bytes32 nullifier) external
            function markFilled(bytes32 intentId, bytes32[] calldata merkleProof, uint256 leafIndex) external
            function syncDestChainRoot(uint32 chainId, bytes32 root) external
            function refund(bytes32 intentId) external
        ]"#
    );

    abigen!(
        EthSettlement,
        r#"[
            function fillIntent(bytes32 intentId, bytes32 commitment, uint32 sourceChain, address token, uint256 amount, bytes32 sourceRoot, bytes32[] calldata merkleProof, uint256 leafIndex) external
            function claimWithdrawal(bytes32 intentId, bytes32 nullifier, address recipient, bytes32 secret, bytes calldata claimAuth) external
            function syncSourceChainRoot(uint32 chainId, bytes32 root) external
            function getMerkleRoot() external view returns (bytes32)
        ]"#
    );
}

use ethereum_contracts::{EthIntentPool, EthSettlement};

pub type EthClient = SignerMiddleware<Provider<Http>, LocalWallet>;

impl EthereumRelayer {
    pub async fn new(config: EthereumConfig, database: Arc<Database>) -> Result<Self> {
        config.validate()?;
        info!("🔗 Initializing Ethereum relayer");

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

        let intent_pool = EthIntentPool::new(intent_pool_address, client.clone());
        let settlement = EthSettlement::new(settlement_address, client.clone());

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
            .map_err(|e| anyhow!("Ethereum RPC unhealthy: {}", e))?;
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
        info!("🔨 Creating intent on Ethereum");

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

    pub async fn fill_intent(
        &self,
        intent_id: &str,
        commitment: &str,
        source_chain: u32,
        token: &str,
        amount: &str,
        source_root: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> Result<String> {
        info!("🔨 Filling intent on Ethereum");

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
                let decoded =
                    hex::decode(&p[2..]).map_err(|e| anyhow!("Invalid proof hex: {}", e))?;
                let array: [u8; 32] = decoded
                    .try_into()
                    .map_err(|_| anyhow!("Invalid proof element length"))?;
                Ok(array)
            })
            .collect::<Result<Vec<[u8; 32]>>>()?;

        let tx = self.settlement.fill_intent(
            intent_id_bytes,
            commitment_bytes,
            source_chain,
            token_address,
            amount_u256,
            source_root_bytes,
            proof,
            U256::from(leaf_index),
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
        info!("🔓 Claiming withdrawal on Ethereum");

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

    pub async fn mark_filled(
        &self,
        intent_id: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> Result<String> {
        info!("✅ Marking intent filled on Ethereum");

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

    pub async fn refund_intent(&self, intent_id: &str) -> Result<String> {
        info!("♻️ Refunding intent on Ethereum");

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

    pub async fn get_merkle_root(&self) -> Result<String> {
        let root = self
            .settlement
            .get_merkle_root()
            .call()
            .await
            .map_err(|e| anyhow!("Failed to get merkle root: {}", e))?;

        Ok(format!("0x{}", hex::encode(root)))
    }

    pub async fn sync_source_chain_root(&self, chain_id: u32, root: String) -> Result<String> {
        info!("🌳 Syncing source chain {} root on Ethereum", chain_id);

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

    pub async fn sync_dest_chain_root(&self, chain_id: u32, root: [u8; 32]) -> Result<String> {
        info!("🌳 Syncing dest chain {} root on Mantle", chain_id);

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

impl ChainRelayer for EthereumRelayer {
    fn get_merkle_root(&self) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { self.get_merkle_root().await }
    }

    fn sync_source_chain_root(
        &self,
        chain_id: u32,
        root: String,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { self.sync_source_chain_root(chain_id, root).await }
    }

    fn sync_dest_chain_root(
        &self,
        chain_id: u32,
        root: [u8; 32],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        async move { self.sync_dest_chain_root(chain_id, root).await }
    }

    fn fill_intent(
        &self,
        intent_id: &str,
        commitment: &str,
        source_chain: u32,
        token: &str,
        amount: &str,
        source_root: &str,
        merkle_path: &[String],
        leaf_index: u32,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let intent_id = intent_id.to_string();
        let commitment = commitment.to_string();
        let token = token.to_string();
        let amount = amount.to_string();
        let source_root = source_root.to_string();
        let merkle_path = merkle_path.to_vec();

        async move {
            self.fill_intent(
                &intent_id,
                &commitment,
                source_chain,
                &token,
                &amount,
                &source_root,
                &merkle_path,
                leaf_index,
            )
            .await
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
            self.mark_filled(&intent_id, &merkle_path, leaf_index)
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
            self.refund_intent(&intent_id)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }
}
