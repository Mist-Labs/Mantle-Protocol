use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::time;

use crate::models::model::{BridgeMetrics, IntentOperationState};
use crate::{
    database::database::Database,
    ethereum::relayer::{EthClient, ethereum_contracts},
    mantle::relayer::{MantleClient, mantle_contracts},
    merkle_manager::merkle_manager::MerkleTreeManager,
    models::model::{DatabaseConfig, ServerConfig},
};
use tokio::sync::RwLock;

pub struct BridgeCoordinator {
    pub ethereum_relayer: Arc<EthereumRelayer>,
    pub mantle_relayer: Arc<MantleRelayer>,
    pub database: Arc<Database>,
    pub merkle_tree_manager: Arc<MerkleTreeManager>,
    pub metrics: Arc<RwLock<BridgeMetrics>>,
    pub operation_states: Arc<RwLock<HashMap<String, IntentOperationState>>>,
    pub start_time: time::Instant,
}

pub struct EthereumRelayer {
    pub client: Arc<EthClient>,
    pub intent_pool: ethereum_contracts::EthIntentPool<EthClient>,
    pub settlement: ethereum_contracts::EthSettlement<EthClient>,
    pub database: Arc<Database>,
    pub chain_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub private_key: String,
    pub intent_pool_address: String,
    pub settlement_address: String,
    pub chain_id: u32,
}

pub struct MantleRelayer {
    pub client: Arc<MantleClient>,
    pub intent_pool: mantle_contracts::MantleIntentPool<MantleClient>,
    pub settlement: mantle_contracts::MantleSettlement<MantleClient>,
    pub database: Arc<Database>,
    pub chain_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MantleConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub private_key: String,
    pub intent_pool_address: String,
    pub settlement_address: String,
    pub chain_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ethereum: EthereumConfig,
    pub mantle: MantleConfig,
    pub relayer_address: String,
    pub fee_collector: String,
}
