use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::database::model::BridgeStats;

// ============================================================================
// BRIDGE REQUEST/RESPONSE MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct InitiateBridgeRequest {
    pub user_address: String,
    pub source_chain: String,
    pub dest_chain: String,
    pub source_token: String,
    pub dest_token: String,
    pub amount: String,
    pub commitment: String,
    pub refund_address: String,
    pub secret: String,           
    pub nullifier: String,        
    pub claim_auth: String,      
    pub recipient: String,
}

#[derive(Debug, Serialize)]
pub struct InitiateBridgeResponse {
    pub success: bool,
    pub intent_id: String,
    pub commitment: String,
    pub message: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IntentStatusResponse {
    pub intent_id: String,
    pub status: String,
    pub source_chain: String,
    pub dest_chain: String,
    pub source_token: String,
    pub dest_token: String,
    pub amount: String,
    pub commitment: Option<String>,
    pub dest_fill_txid: Option<String>,
    pub source_complete_txid: Option<String>,
    pub deadline: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub has_privacy: bool,
}

// ============================================================================
// INDEXER EVENT MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct IndexerEventRequest {
    pub event_type: String,
    pub chain: String,
    pub transaction_hash: String,
    pub block_number: Option<u64>,
    pub timestamp: i64,

    // Intent-related fields
    pub intent_id: Option<String>,
    pub commitment: Option<String>,
    pub nullifier: Option<String>,
    pub solver: Option<String>,

    // Root sync fields
    pub root: Option<String>,
    pub dest_chain_id: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct IndexerEventResponse {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

// ============================================================================
// PRICE FEED MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PriceRequest {
    pub from_symbol: String,
    pub to_symbol: String,
    pub amount: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PriceResponse {
    pub from_symbol: String,
    pub to_symbol: String,
    pub rate: f64,
    pub amount: Option<f64>,
    pub converted_amount: Option<f64>,
    pub timestamp: i64,
    pub sources: Vec<PriceSourceInfo>,
}

#[derive(Debug, Serialize)]
pub struct PriceSourceInfo {
    pub source: String,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub struct AllPricesResponse {
    pub status: String,
    pub timestamp: i64,
    pub prices: HashMap<String, f64>,
}

#[derive(Debug, Deserialize)]
pub struct ConvertRequest {
    pub from_symbol: String,
    pub to_symbol: String,
    pub amount: f64,
}

#[derive(Debug, Serialize)]
pub struct ConvertResponse {
    pub from_symbol: String,
    pub to_symbol: String,
    pub input_amount: f64,
    pub output_amount: f64,
    pub rate: f64,
    pub timestamp: i64,
}

// ============================================================================
// STATS MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub status: String,
    pub data: BridgeStats,
}
