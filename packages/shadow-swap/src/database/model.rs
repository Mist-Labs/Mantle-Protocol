use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::{
    model::{Intent, IntentPrivacyParams, IntentStatus},
    schema::{
        bridge_events, chain_transactions, indexer_checkpoints, intent_privacy_params, intents,
        merkle_nodes, merkle_roots, merkle_trees,
    },
};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = intents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbIntent {
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
    pub source_complete_txid: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: i64,
    pub refund_address: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = intents)]
pub struct NewIntent<'a> {
    pub id: &'a str,
    pub user_address: &'a str,
    pub source_chain: &'a str,
    pub dest_chain: &'a str,
    pub source_token: &'a str,
    pub dest_token: &'a str,
    pub amount: &'a str,
    pub dest_amount: &'a str,
    pub source_commitment: Option<&'a str>,
    pub dest_fill_txid: Option<&'a str>,
    pub source_complete_txid: Option<&'a str>,
    pub status: &'a str,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: i64,
    pub refund_address: Option<&'a str>,
}
// ==================== Intent Privacy Params ====================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = intent_privacy_params)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbIntentPrivacyParams {
    pub intent_id: String,
    pub commitment: Option<String>,
    pub nullifier: Option<String>,
    pub secret: Option<String>,
    pub recipient: Option<String>,
    pub claim_signature: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = intent_privacy_params)]
pub struct NewIntentPrivacyParams<'a> {
    pub intent_id: &'a str,
    pub commitment: Option<&'a str>,
    pub nullifier: Option<&'a str>,
    pub secret: Option<&'a str>,
    pub recipient: Option<&'a str>,
    pub claim_signature: Option<&'a str>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ==================== Chain Transactions ====================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = chain_transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbChainTransaction {
    pub id: i32,
    pub intent_id: String,
    pub chain_id: i32,
    pub tx_type: String,
    pub tx_hash: String,
    pub status: String,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = chain_transactions)]
pub struct NewChainTransaction<'a> {
    pub intent_id: &'a str,
    pub chain_id: i32,
    pub tx_type: &'a str,
    pub tx_hash: &'a str,
    pub status: &'a str,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
}

// ==================== Bridge Events ====================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = bridge_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbBridgeEvent {
    pub id: i32,
    pub event_id: String,
    pub intent_id: Option<String>,
    pub event_type: String,
    pub event_data: serde_json::Value,
    pub chain_id: i32,
    pub block_number: i64,
    pub transaction_hash: String,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = bridge_events)]
pub struct NewBridgeEvent<'a> {
    pub event_id: &'a str,
    pub intent_id: Option<&'a str>,
    pub event_type: &'a str,
    pub event_data: serde_json::Value,
    pub chain_id: i32,
    pub block_number: i64,
    pub transaction_hash: &'a str,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ==================== Indexer Checkpoints ====================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = indexer_checkpoints)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbIndexerCheckpoint {
    pub chain: String,
    pub last_block: i32,
    pub updated_at: DateTime<Utc>,
}

// ==================== Helper Structs ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStats {
    pub total_intents: u64,
    pub pending_intents: u64,
    pub filled_intents: u64,
    pub completed_intents: u64,
    pub failed_intents: u64,
    pub refunded_intents: u64,
    pub ethereum_to_mantle: u64,
    pub mantle_to_ethereum: u64,
    pub total_volume_by_token: HashMap<String, String>,
}

// ==================== Merkle Structs ====================
#[derive(Queryable, Debug, Clone, Serialize, Deserialize, Selectable)]
#[diesel(table_name = merkle_trees)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbMerkleTree {
    pub tree_id: i32,
    pub tree_name: String,
    pub depth: i32,
    pub root: String,
    pub leaf_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = merkle_trees)]
pub struct NewMerkleTree<'a> {
    pub tree_name: &'a str,
    pub depth: i32,
    pub root: &'a str,
    pub leaf_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Debug, Clone, Serialize, Deserialize, Selectable)]
#[diesel(table_name = merkle_nodes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbMerkleNode {
    pub node_id: i32,
    pub tree_id: i32,
    pub level: i32,
    pub node_index: i64,
    pub hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = merkle_nodes)]
pub struct NewMerkleNode<'a> {
    pub tree_id: i32,
    pub level: i32,
    pub node_index: i64,
    pub hash: &'a str,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Debug, Clone, Serialize, Deserialize, Selectable)]
#[diesel(table_name = merkle_roots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbMerkleRoot {
    pub tree_id: String,
    pub root_hash: String,
    pub leaf_count: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = merkle_roots)]
pub struct NewMerkleRoot<'a> {
    pub tree_id: &'a str,
    pub root_hash: &'a str,
    pub leaf_count: i64,
    pub updated_at: DateTime<Utc>,
}

impl IntentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Pending => "pending",
            Self::Committed => "committed",
            Self::Filled => "filled",
            Self::Completed => "completed",
            Self::Refunded => "refunded",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match s {
            "created" => Ok(Self::Created),
            "filled" => Ok(Self::Filled),
            "completed" => Ok(Self::Completed),
            "refunded" => Ok(Self::Refunded),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Invalid intent status: {}", s).into()),
        }
    }
}

impl From<DbIntent> for Intent {
    fn from(db: DbIntent) -> Self {
        Self {
            id: db.id,
            user_address: db.user_address,
            source_chain: db.source_chain,
            dest_chain: db.dest_chain,
            source_token: db.source_token,
            dest_token: db.dest_token,
            amount: db.amount,
            dest_amount: db.dest_amount,
            source_commitment: db.source_commitment,
            dest_fill_txid: db.dest_fill_txid,
            source_complete_txid: db.source_complete_txid,
            status: IntentStatus::from_str(&db.status).unwrap_or(IntentStatus::Failed),
            created_at: db.created_at,
            updated_at: db.updated_at,
            deadline: db.deadline as u64,
            refund_address: db.refund_address,
        }
    }
}

impl<'a> From<&'a Intent> for NewIntent<'a> {
    fn from(intent: &'a Intent) -> Self {
        Self {
            id: &intent.id,
            user_address: &intent.user_address,
            source_chain: &intent.source_chain,
            dest_chain: &intent.dest_chain,
            source_token: &intent.source_token,
            dest_token: &intent.dest_token,
            amount: &intent.amount,
            dest_amount: &intent.dest_amount,
            source_commitment: intent.source_commitment.as_deref(),
            dest_fill_txid: intent.dest_fill_txid.as_deref(),
            source_complete_txid: intent.source_complete_txid.as_deref(),
            status: intent.status.as_str(),
            created_at: intent.created_at,
            updated_at: intent.updated_at,
            deadline: intent.deadline as i64,
            refund_address: intent.refund_address.as_deref(),
        }
    }
}

impl From<DbIntentPrivacyParams> for IntentPrivacyParams {
    fn from(db: DbIntentPrivacyParams) -> Self {
        Self {
            intent_id: db.intent_id,
            commitment: db.commitment,
            nullifier: db.nullifier,
            secret: db.secret,
            recipient: db.recipient,
            claim_signature: db.claim_signature,
        }
    }
}
