use std::collections::HashMap;

use ethers::types::{Address, H256, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedToken {
    ETH,
    USDC,
    USDT,
    WETH,
    MNT,
}

#[derive(Debug, Clone)]
pub struct SolverConfig {
    // Capital Management per token
    pub max_capital_per_fill: HashMap<SupportedToken, U256>,
    pub min_capital_reserve: HashMap<SupportedToken, U256>,
    pub max_concurrent_fills: usize,

    // Risk Parameters
    pub min_profit_bps: u16,
    pub source_confirmations_required: u64,
    pub max_intent_age_secs: u64,

    // Chain Configuration
    pub ethereum_rpc: String,
    pub mantle_rpc: String,
    pub ethereum_settlement: Address,
    pub mantle_settlement: Address,
    pub ethereum_intent_pool: Address,
    pub mantle_intent_pool: Address,

    // Chain IDs
    pub ethereum_chain_id: u64,
    pub mantle_chain_id: u64,

    // Solver Identity
    pub solver_address: Address,
    pub solver_private_key: String,

    // Gas Configuration
    pub max_gas_price_gwei: U256,
    pub priority_fee_gwei: U256,

    // Monitoring
    pub health_check_interval_secs: u64,
    pub balance_check_interval_secs: u64,
}

#[derive(Debug, Clone)]
pub struct DetectedIntent {
    pub intent_id: H256,
    pub commitment: H256,
    pub token: Address,
    pub token_type: SupportedToken,
    pub amount: U256,
    pub source_chain: u32,
    pub dest_chain: u32,
    pub source_block: u64,
    pub detected_at: u64,
}

#[derive(Debug, Clone)]
pub struct FillOpportunity {
    pub intent: DetectedIntent,
    pub estimated_profit: U256,
    pub profit_bps: u16,
    pub risk_score: u8,
    pub capital_required: U256,
    pub gas_estimate: U256,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FillStatus {
    Pending,
    Confirmed,
    Claimed,
    Failed,
    Expired,
}

#[derive(Debug, Clone)]
pub struct ActiveFill {
    pub intent_id: H256,
    pub tx_hash: H256,
    pub amount: U256,
    pub token: Address,
    pub token_type: SupportedToken,
    pub filled_at: u64,
    pub confirmed_at: Option<u64>,
    pub status: FillStatus,
    pub dest_chain: u32,
}

// ============================================================================
// SOLVER METRICS
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct SolverMetrics {
    pub total_intents_evaluated: u64,
    pub total_fills_attempted: u64,
    pub successful_fills: u64,
    pub failed_fills: u64,
    pub total_profit_earned: HashMap<SupportedToken, U256>,
    pub capital_deployed: HashMap<SupportedToken, U256>,
    pub capital_available: HashMap<(SupportedToken, u64), U256>,
    pub active_fills_count: usize,
    pub average_fill_time_secs: f64,
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MetricsResponse {
    pub total_intents_evaluated: u64,
    pub total_fills_attempted: u64,
    pub successful_fills: u64,
    pub failed_fills: u64,
    pub active_fills_count: usize,
    pub average_fill_time_secs: f64,
    pub capital_deployed: HashMap<String, String>,
    pub capital_available: HashMap<String, String>,
    pub total_profit_earned: HashMap<String, String>,
    pub last_error: Option<String>,
}
