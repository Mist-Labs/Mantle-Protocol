use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::relay_coordinator::model::{EthereumConfig, MantleConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ethereum: EthereumConfig,
    pub mantle: MantleConfig,
    pub relayer_address: String,
    pub fee_collector: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub hmac_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone)]
pub struct MantleIntent {
    pub commitment: String,
    pub block_number: u64,
    pub log_index: u64,
}

#[derive(Debug, Clone)]
pub struct EthereumFill {
    pub intent_id: String,
    pub block_number: u64,
    pub log_index: u64,
}

#[derive(Debug, Clone)]
pub struct EthereumIntent {
    pub commitment: String,
    pub block_number: u64,
    pub log_index: u64,
}

#[derive(Debug, Clone)]
pub struct MantleFill {
    pub intent_id: String,
    pub block_number: u64,
    pub log_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub id: String,
    pub user_address: String,
    pub source_chain: String,
    pub dest_chain: String,
    pub source_token: String,
    pub dest_token: String,
    pub amount: String,
    pub dest_amount: String,
    pub source_commitment: Option<String>,
    pub dest_fill_txid: Option<String>,
    pub dest_registration_txid: Option<String>,
    pub source_complete_txid: Option<String>,
    pub status: IntentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: u64,
    pub refund_address: Option<String>,
    pub solver_address: Option<String>,
    pub block_number: Option<i64>,
    pub log_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPrivacyParams {
    pub intent_id: String,
    pub commitment: Option<String>,
    pub nullifier: Option<String>,
    pub secret: Option<String>,
    pub recipient: Option<String>,
    pub claim_signature: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentStatus {
    Created,
    Committed,
    Registered,
    Pending,
    Filled,
    SolverPaid,
    UserClaimed,
    Refunded,
    Failed,
    Expired,
}

#[derive(Debug, Clone)]
pub struct IntentCreatedEvent {
    pub intent_id: String,
    pub commitment: String,
    pub source_token: String,
    pub source_amount: String,
    pub dest_token: String,
    pub dest_amount: String,
    pub dest_chain: u32,
    pub deadline: Option<u64>,
    pub block_number: Option<u64>,
    pub transaction_hash: Option<String>,
    pub log_index: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct IntentOperationState {
    pub intent_id: String,
    pub direction: BridgeDirection,
    pub status: IntentStatus,
    pub token_info: TokenBridgeInfo,
    pub last_update: u64,
}

#[derive(Debug, Clone)]
pub enum BridgeDirection {
    EthereumToMantle,
    MantleToEthereum,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct BridgeMetrics {
    pub total_intents_processed: u64,
    pub successful_bridges: u64,
    pub failed_intents: u64,
    pub refunded_intents: u64,
    pub ethereum_fills: u64,
    pub mantle_fills: u64,
    pub ethereum_claims: u64,
    pub mantle_claims: u64,
    pub retry_attempts: u64,
    pub last_error: Option<String>,
    pub uptime_seconds: u64,
    pub volumes_by_token: HashMap<TokenType, u128>,
}

#[derive(Debug, Clone)]
pub struct TokenBridgeInfo {
    pub token_type: TokenType,
    pub source_address: String,
    pub dest_address: String,
    pub amount: String,
    pub decimals: u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum TokenType {
    ETH,
    USDC,
    USDT,
    WETH,
    MNT,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum Chain {
//     Ethereum,
//     Mantle,
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum TreeType {
//     Intent,
//     Commitment,
//     Fill,
// }

// impl Chain {
//     pub fn from_str(s: &str) -> Option<Self> {
//         match s.to_lowercase().as_str() {
//             "ethereum" => Some(Self::Ethereum),
//             "mantle" => Some(Self::Mantle),
//             _ => None,
//         }
//     }

//     pub fn as_str(&self) -> &'static str {
//         match self {
//             Self::Ethereum => "ethereum",
//             Self::Mantle => "mantle",
//         }
//     }

//     pub fn chain_id(&self) -> u32 {
//         match self {
//             Self::Ethereum => 11155111,
//             Self::Mantle => 5003,
//         }
//     }
// }

// impl TreeType {
//     pub fn tree_name(&self, chain: Chain) -> String {
//         match (chain, self) {
//             (Chain::Ethereum, TreeType::Intent) => "ethereum_intents".to_string(),
//             (Chain::Ethereum, TreeType::Commitment) => "ethereum_commitments".to_string(),
//             (Chain::Ethereum, TreeType::Fill) => "ethereum_fills".to_string(),
//             (Chain::Mantle, TreeType::Intent) => "mantle_intents".to_string(),
//             (Chain::Mantle, TreeType::Commitment) => "mantle_commitments".to_string(),
//             (Chain::Mantle, TreeType::Fill) => "mantle_fills".to_string(),
//         }
//     }
// }
