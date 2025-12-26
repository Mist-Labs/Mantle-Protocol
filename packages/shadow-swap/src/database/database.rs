use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenv::dotenv;
use tracing::info;

use crate::database::model::{
    BridgeStats, DbBridgeEvent, DbChainTransaction, DbEthereumIntentCreated, DbMantleIntentCreated,
    DbMerkleNode, DbMerkleTree, NewBridgeEvent, NewChainTransaction, NewMerkleNode, NewMerkleTree,
};

use crate::models::model::{EthereumFill, EthereumIntent, MantleFill, MantleIntent};
use crate::models::schema::{
    bridge_events, chain_transactions, ethereum_sepolia_intent_created, indexer_checkpoints,
    mantle_sepolia_intent_created, merkle_trees, root_syncs,
};
use crate::{
    database::model::{DbIntent, DbIntentPrivacyParams, NewIntent, NewIntentPrivacyParams},
    models::{
        model::{Intent, IntentPrivacyParams, IntentStatus},
        schema::{intent_privacy_params, intents},
    },
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
pub const TREE_DEPTH: i32 = 20;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Debug)]
pub enum DatabaseSetupError {
    DbConnectionError(::r2d2::Error),
    DieselError(diesel::result::Error),
    DatabaseUrlNotSet,
    ErrorRunningMigrations,
}

impl std::fmt::Display for DatabaseSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseSetupError::DbConnectionError(e) => {
                write!(f, "Database connection error: {}", e)
            }
            DatabaseSetupError::DieselError(e) => write!(f, "Diesel error: {}", e),
            DatabaseSetupError::DatabaseUrlNotSet => write!(f, "DATABASE_URL not set"),
            DatabaseSetupError::ErrorRunningMigrations => write!(f, "Error running migrations"),
        }
    }
}

impl std::error::Error for DatabaseSetupError {}

#[derive(Clone)]
pub struct Database {
    pub pool: DbPool,
}

impl Database {
    pub fn new(database_url: &str, max_connection: u32) -> Result<Self> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(max_connection)
            .build(manager)
            .context("Failed to create database pool")?;

        Ok(Database { pool })
    }

    pub fn health_check(&self) -> Result<()> {
        let mut conn = self
            .get_connection()
            .context("Database connection failed")?;

        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .context("Database query failed")?;

        Ok(())
    }

    pub fn run_migrations(
        pool: &Pool<ConnectionManager<PgConnection>>,
    ) -> Result<(), DatabaseSetupError> {
        info!("RUNNING MIGRATIONS....");
        let mut conn = pool.get().map_err(DatabaseSetupError::DbConnectionError)?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|_| DatabaseSetupError::ErrorRunningMigrations)?;
        info!("MIGRATIONS COMPLETED....");
        Ok(())
    }

    pub fn from_env() -> Result<Self> {
        dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?;

        let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(max_connections)
            .build(manager)
            .context("Failed to create database pool")?;

        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "prod".into());
        if env == "dev" {
            Database::run_migrations(&pool)?;
        }

        Ok(Database { pool })
    }

    pub fn get_connection(
        &self,
    ) -> Result<r2d2::PooledConnection<ConnectionManager<PgConnection>>> {
        self.pool.get().context("Failed to get database connection")
    }

    // ==================== Intent CRUD Operations ====================

    pub fn create_intent(&self, intent: &Intent) -> Result<()> {
        let mut conn = self.get_connection()?;

        let new_intent = NewIntent {
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
            dest_registration_txid: intent.dest_registration_txid.as_deref(),
            source_complete_txid: intent.source_complete_txid.as_deref(),
            status: intent.status.as_str(),
            created_at: intent.created_at,
            updated_at: intent.updated_at,
            deadline: intent.deadline as i64,
            refund_address: intent.refund_address.as_deref(),
        };

        diesel::insert_into(intents::table)
            .values(&new_intent)
            .execute(&mut conn)
            .context("Failed to create intent")?;

        Ok(())
    }

    pub fn create_intent_with_privacy(
        &self,
        intent: &Intent,
        privacy_params: &IntentPrivacyParams,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;

        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let new_intent = NewIntent {
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
                dest_registration_txid: intent.dest_registration_txid.as_deref(),
                source_complete_txid: intent.source_complete_txid.as_deref(),
                status: intent.status.as_str(),
                created_at: intent.created_at,
                updated_at: intent.updated_at,
                deadline: intent.deadline as i64,
                refund_address: intent.refund_address.as_deref(),
            };

            diesel::insert_into(intents::table)
                .values(&new_intent)
                .execute(conn)
                .context("Failed to insert intent")?;

            let new_privacy = NewIntentPrivacyParams {
                intent_id: &intent.id,
                commitment: privacy_params.commitment.as_deref(),
                nullifier: privacy_params.nullifier.as_deref(),
                secret: privacy_params.secret.as_deref(),
                recipient: privacy_params.recipient.as_deref(),
                claim_signature: privacy_params.claim_signature.as_deref(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            diesel::insert_into(intent_privacy_params::table)
                .values(&new_privacy)
                .execute(conn)
                .context("Failed to insert privacy params")?;

            Ok(())
        })?;

        Ok(())
    }

    pub fn update_intent_status(&self, intent_id: &str, status: IntentStatus) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(intents::table.filter(intents::id.eq(intent_id)))
            .set((
                intents::status.eq(status.as_str()),
                intents::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update intent status")?;

        Ok(())
    }

    pub fn update_intent_secret(&self, intent_id: &str, secret: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(
            intent_privacy_params::table.filter(intent_privacy_params::intent_id.eq(intent_id)),
        )
        .set((
            intent_privacy_params::secret.eq(secret),
            intent_privacy_params::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .context("Failed to update intent secret")?;

        Ok(())
    }

    pub fn update_source_commitment(&self, intent_id: &str, commitment: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(intents::table.filter(intents::id.eq(intent_id)))
            .set((
                intents::source_commitment.eq(commitment),
                intents::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update source commitment")?;

        Ok(())
    }

    pub fn update_dest_fill_txid(&self, intent_id: &str, txid: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(intents::table.filter(intents::id.eq(intent_id)))
            .set((
                intents::dest_fill_txid.eq(txid),
                intents::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update dest fill txid")?;

        Ok(())
    }

    pub fn get_intent_by_id(&self, intent_id: &str) -> Result<Option<Intent>> {
        let mut conn = self.get_connection()?;

        let result = intents::table
            .filter(intents::id.eq(intent_id))
            .select(DbIntent::as_select())
            .first::<DbIntent>(&mut conn)
            .optional()
            .context("Failed to get intent by id")?;

        Ok(result.map(db_intent_to_model))
    }

    pub fn get_pending_intents(&self) -> Result<Vec<Intent>> {
        let mut conn = self.get_connection()?;

        let results = intents::table
            .filter(intents::status.eq_any(vec!["created", "filled"]))
            .select(DbIntent::as_select())
            .load::<DbIntent>(&mut conn)
            .context("Failed to get pending intents")?;

        Ok(results.into_iter().map(db_intent_to_model).collect())
    }

    pub fn get_intents_awaiting_secret(&self) -> Result<Vec<Intent>> {
        let mut conn = self.get_connection()?;

        let results = intents::table
            .inner_join(
                intent_privacy_params::table.on(intent_privacy_params::intent_id.eq(intents::id)),
            )
            .filter(intents::status.eq("filled"))
            .filter(intent_privacy_params::secret.is_null())
            .select(DbIntent::as_select())
            .load::<DbIntent>(&mut conn)
            .context("Failed to get intents awaiting secret")?;

        Ok(results.into_iter().map(db_intent_to_model).collect())
    }

    pub fn get_intent_privacy_params(&self, intent_id: &str) -> Result<IntentPrivacyParams> {
        let mut conn = self.get_connection()?;

        let params = intent_privacy_params::table
            .filter(intent_privacy_params::intent_id.eq(intent_id))
            .select(DbIntentPrivacyParams::as_select())
            .first::<DbIntentPrivacyParams>(&mut conn)
            .context("Failed to get intent privacy params")?;

        Ok(IntentPrivacyParams::from(params))
    }

    pub fn list_intents(
        &self,
        status_filter: Option<&str>,
        chain_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Intent>> {
        let mut conn = self.get_connection()?;

        let mut query = intents::table.into_boxed();

        if let Some(status) = status_filter {
            query = query.filter(intents::status.eq(status));
        }

        if let Some(chain) = chain_filter {
            query = query.filter(
                intents::source_chain
                    .eq(chain)
                    .or(intents::dest_chain.eq(chain)),
            );
        }

        let results = query
            .order(intents::created_at.desc())
            .limit(limit as i64)
            .select(DbIntent::as_select())
            .load::<DbIntent>(&mut conn)
            .context("Failed to list intents")?;

        Ok(results.into_iter().map(db_intent_to_model).collect())
    }

    pub fn store_intent_privacy_params(
        &self,
        intent_id: &str,
        commitment: &str,
        secret: &str,
        nullifier: &str,
        claim_auth: &str,
        recipient: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;

        let new_params = NewIntentPrivacyParams {
            intent_id,
            commitment: Some(commitment),
            secret: Some(secret),
            nullifier: Some(nullifier),
            claim_signature: Some(claim_auth),
            recipient: Some(recipient),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        diesel::insert_into(intent_privacy_params::table)
            .values(&new_params)
            .on_conflict(intent_privacy_params::intent_id)
            .do_update()
            .set((
                intent_privacy_params::commitment.eq(Some(commitment)),
                intent_privacy_params::secret.eq(Some(secret)),
                intent_privacy_params::nullifier.eq(Some(nullifier)),
                intent_privacy_params::claim_signature.eq(Some(claim_auth)),
                intent_privacy_params::recipient.eq(Some(recipient)),
                intent_privacy_params::updated_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to store/update privacy params")?;

        Ok(())
    }

    pub fn update_privacy_params(
        &self,
        intent_id: &str,
        privacy_params: &IntentPrivacyParams,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(
            intent_privacy_params::table.filter(intent_privacy_params::intent_id.eq(intent_id)),
        )
        .set((
            intent_privacy_params::commitment.eq(privacy_params.commitment.as_deref()),
            intent_privacy_params::nullifier.eq(privacy_params.nullifier.as_deref()),
            intent_privacy_params::secret.eq(privacy_params.secret.as_deref()),
            intent_privacy_params::recipient.eq(privacy_params.recipient.as_deref()),
            intent_privacy_params::claim_signature.eq(privacy_params.claim_signature.as_deref()),
            intent_privacy_params::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .context("Failed to update privacy params")?;

        Ok(())
    }

    pub fn update_intent(&self, intent: &Intent) -> Result<()> {
        let mut conn = self.get_connection()?;

        let new_intent = NewIntent {
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
            dest_registration_txid: intent.dest_fill_txid.as_deref(),
            source_complete_txid: intent.source_complete_txid.as_deref(),
            status: intent.status.as_str(),
            created_at: intent.created_at,
            updated_at: intent.updated_at,
            deadline: intent.deadline as i64,
            refund_address: intent.refund_address.as_deref(),
        };

        diesel::update(intents::table.filter(intents::id.eq(&intent.id)))
            .set((
                intents::source_commitment.eq(new_intent.source_commitment),
                intents::dest_fill_txid.eq(new_intent.dest_fill_txid),
                intents::source_complete_txid.eq(new_intent.source_complete_txid),
                intents::status.eq(new_intent.status),
                intents::updated_at.eq(new_intent.updated_at),
            ))
            .execute(&mut conn)
            .context("Failed to update intent")?;

        Ok(())
    }

    pub fn record_intent_event(
        &self,
        intent_id: &str,
        event_type: &str,
        chain: &str,
        tx_hash: &str,
        block_number: u64,
    ) -> Result<()> {
        let chain_id = match chain {
            "ethereum" => 11155111, // Ethereum Sepolia
            "mantle" => 5003,       // Mantle Sepolia
            _ => 0,
        };

        let event_data = serde_json::json!({
            "intent_id": intent_id,
            "event_type": event_type,
            "chain": chain,
        });

        let event_id = format!(
            "{}_{}_{}_{}",
            event_type,
            intent_id,
            chain,
            chrono::Utc::now().timestamp()
        );

        self.store_bridge_event(
            &event_id,
            Some(intent_id),
            event_type,
            event_data,
            chain_id,
            block_number,
            tx_hash,
        )
    }

    pub fn update_dest_registration_txid(
        &self,
        intent_id: &str,
        txid: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::models::schema::intents::dsl::*;

        let mut conn = self.pool.get()?;

        diesel::update(intents.filter(id.eq(intent_id)))
            .set(dest_registration_txid.eq(Some(txid)))
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn record_nullifier_usage(
        &self,
        nullifier: &str,
        intent_id: &str,
        tx_hash: &str,
    ) -> Result<()> {
        let event_data = serde_json::json!({
            "nullifier": nullifier,
            "intent_id": intent_id,
            "used_at": chrono::Utc::now().to_rfc3339(),
        });

        let event_id = format!("nullifier_{}_{}", nullifier, chrono::Utc::now().timestamp());

        self.store_bridge_event(
            &event_id,
            Some(intent_id),
            "nullifier_used",
            event_data,
            0,
            0,
            tx_hash,
        )
    }

    // ==================== Chain Transaction Logging ====================

    pub fn log_chain_transaction(
        &self,
        intent_id: &str,
        chain_id: u32,
        tx_type: &str,
        tx_hash: &str,
        status: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;
        let timestamp = Utc::now().timestamp();

        let new_tx = NewChainTransaction {
            intent_id,
            chain_id: chain_id as i32,
            tx_type,
            tx_hash,
            status,
            timestamp,
            created_at: Utc::now(),
        };

        diesel::insert_into(chain_transactions::table)
            .values(&new_tx)
            .on_conflict(chain_transactions::tx_hash)
            .do_update()
            .set((
                chain_transactions::status.eq(status),
                chain_transactions::timestamp.eq(timestamp),
            ))
            .execute(&mut conn)
            .context("Failed to log chain transaction")?;

        Ok(())
    }

    pub fn get_transaction_by_hash(&self, tx_hash: &str) -> Result<Option<DbChainTransaction>> {
        let mut conn = self.get_connection()?;

        let result = chain_transactions::table
            .filter(chain_transactions::tx_hash.eq(tx_hash))
            .select(DbChainTransaction::as_select())
            .first::<DbChainTransaction>(&mut conn)
            .optional()
            .context("Failed to get transaction by hash")?;

        Ok(result)
    }

    // ==================== Bridge Events ====================

    pub fn store_bridge_event(
        &self,
        event_id: &str,
        intent_id: Option<&str>,
        event_type: &str,
        event_data: serde_json::Value,
        chain_id: u32,
        block_number: u64,
        transaction_hash: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;

        let new_event = NewBridgeEvent {
            event_id,
            intent_id,
            event_type,
            event_data,
            chain_id: chain_id as i32,
            block_number: block_number as i64,
            transaction_hash,
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        diesel::insert_into(bridge_events::table)
            .values(&new_event)
            .execute(&mut conn)
            .context("Failed to store bridge event")?;

        Ok(())
    }

    pub fn get_bridge_event_by_nullifier(
        &self,
        nullifier: &str,
        event_type: &str,
        chain_id: u32,
    ) -> Result<Option<serde_json::Value>> {
        let mut conn = self.get_connection()?;

        let result = bridge_events::table
            .filter(bridge_events::event_type.eq(event_type))
            .filter(bridge_events::chain_id.eq(chain_id as i32))
            .filter(
                bridge_events::event_data
                    .retrieve_as_text("nullifier")
                    .eq(nullifier),
            )
            .select(bridge_events::event_data)
            .order(bridge_events::created_at.desc())
            .first::<serde_json::Value>(&mut conn)
            .optional()
            .context("Failed to get bridge event by nullifier")?;

        Ok(result)
    }

    // ==================== Indexer Checkpoints ====================

    pub fn save_indexer_checkpoint(&self, chain: &str, height: u32) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::insert_into(indexer_checkpoints::table)
            .values((
                indexer_checkpoints::chain.eq(chain),
                indexer_checkpoints::last_block.eq(height as i32),
                indexer_checkpoints::updated_at.eq(Utc::now()),
            ))
            .on_conflict(indexer_checkpoints::chain)
            .do_update()
            .set((
                indexer_checkpoints::last_block.eq(height as i32),
                indexer_checkpoints::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to save indexer checkpoint")?;

        Ok(())
    }

    pub fn get_indexer_checkpoint(&self, chain: &str) -> Result<Option<u32>> {
        let mut conn = self.get_connection()?;

        let result = indexer_checkpoints::table
            .filter(indexer_checkpoints::chain.eq(chain))
            .select(indexer_checkpoints::last_block)
            .first::<i32>(&mut conn)
            .optional()
            .context("Failed to get indexer checkpoint")?;

        Ok(result.map(|b| b as u32))
    }

    // ==================== Merkle Trees ====================

    pub fn create_merkle_tree(&self, tree_name: &str, depth: i32) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        let new_tree = NewMerkleTree {
            tree_name,
            depth,
            root: "0x0000000000000000000000000000000000000000000000000000000000000000",
            leaf_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        diesel::insert_into(merkle_trees::table)
            .values(&new_tree)
            .execute(&mut conn)
            .context("Failed to create merkle tree")?;

        Ok(())
    }

    pub fn ensure_merkle_tree(&self, tree_name: &str, depth: i32) -> Result<DbMerkleTree> {
        if let Some(tree) = self.get_merkle_tree_by_name(tree_name)? {
            return Ok(tree);
        }

        self.create_merkle_tree(tree_name, depth)?;

        self.get_merkle_tree_by_name(tree_name)?
            .ok_or_else(|| anyhow!("Failed to ensure merkle tree {}", tree_name))
    }

    pub fn get_merkle_tree_by_name(&self, tree_name: &str) -> Result<Option<DbMerkleTree>> {
        let mut conn = self.get_connection()?;

        let tree = merkle_trees::table
            .filter(merkle_trees::tree_name.eq(tree_name))
            .select(DbMerkleTree::as_select())
            .first::<DbMerkleTree>(&mut conn)
            .optional()
            .context("Failed to get merkle tree by name")?;

        Ok(tree)
    }

    pub fn update_merkle_root(&self, tree_id: i32, root: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree_id)))
            .set((
                merkle_trees::root.eq(root),
                merkle_trees::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update merkle root")?;

        Ok(())
    }

    pub fn increment_leaf_count(&self, tree_id: i32, count: i64) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree_id)))
            .set((
                merkle_trees::leaf_count.eq(merkle_trees::leaf_count + count),
                merkle_trees::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to increment leaf count")?;

        Ok(())
    }

    pub fn get_ethereum_commitment_tree_size(&self) -> Result<usize> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        Ok(tree.leaf_count as usize)
    }

    pub fn add_to_ethereum_commitment_tree(&self, _commitment: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    pub fn set_ethereum_commitment_node(
        &self,
        level: usize,
        index: usize,
        hash: &str,
    ) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        self.store_merkle_node(tree.tree_id, level as i32, index as i64, hash)?;
        Ok(())
    }

    pub fn get_ethereum_commitment_node(
        &self,
        level: usize,
        index: usize,
    ) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        let node = self.get_merkle_node(tree.tree_id, level as i32, index as i64)?;
        Ok(node.map(|n| n.hash))
    }

    pub fn clear_ethereum_commitment_nodes(&self) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)
            .context("Failed to clear ethereum commitment nodes")?;

        Ok(())
    }

    pub fn clear_ethereum_commitment_tree(&self) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        self.clear_ethereum_commitment_nodes()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree.tree_id)))
            .set(merkle_trees::leaf_count.eq(0))
            .execute(&mut conn)
            .context("Failed to reset ethereum commitment tree leaf count")?;

        Ok(())
    }

    pub fn get_ethereum_commitment_tree(&self) -> Result<Vec<String>> {
        let _tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;
        let commitments = self.get_all_ethereum_commitments()?;

        Ok(commitments)
    }

    // ==================== Merkle Nodes ====================

    pub fn store_merkle_node(
        &self,
        tree_id: i32,
        level: i32,
        node_index: i64,
        hash: &str,
    ) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;
        let now = Utc::now();

        let node = NewMerkleNode {
            tree_id,
            level,
            node_index,
            hash,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(merkle_nodes::table)
            .values(&node)
            .on_conflict((
                merkle_nodes::tree_id,
                merkle_nodes::level,
                merkle_nodes::node_index,
            ))
            .do_update()
            .set((
                merkle_nodes::hash.eq(hash),
                merkle_nodes::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .context("Failed to store merkle node")?;

        Ok(())
    }

    pub fn get_merkle_node(
        &self,
        tree_id: i32,
        level: i32,
        node_index: i64,
    ) -> Result<Option<DbMerkleNode>> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let node = merkle_nodes::table
            .filter(merkle_nodes::tree_id.eq(tree_id))
            .filter(merkle_nodes::level.eq(level))
            .filter(merkle_nodes::node_index.eq(node_index))
            .select(DbMerkleNode::as_select())
            .first::<DbMerkleNode>(&mut conn)
            .optional()
            .context("Failed to get merkle node")?;

        Ok(node)
    }

    pub fn get_merkle_nodes_by_level(&self, tree_id: i32, level: i32) -> Result<Vec<DbMerkleNode>> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let nodes = merkle_nodes::table
            .filter(merkle_nodes::tree_id.eq(tree_id))
            .filter(merkle_nodes::level.eq(level))
            .order(merkle_nodes::node_index.asc())
            .select(DbMerkleNode::as_select())
            .load::<DbMerkleNode>(&mut conn)
            .context("Failed to get merkle nodes by level")?;

        Ok(nodes)
    }

    pub fn delete_merkle_tree(&self, tree_id: i32) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        diesel::delete(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree_id)))
            .execute(&mut conn)
            .context("Failed to delete merkle tree")?;

        Ok(())
    }

    pub fn delete_merkle_tree_by_name(&self, tree_name: &str) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        diesel::delete(merkle_trees::table.filter(merkle_trees::tree_name.eq(tree_name)))
            .execute(&mut conn)
            .context("Failed to delete merkle tree by name")?;

        Ok(())
    }

    // ==================== Merkle Tree Operations ====================

    /// Get Mantle tree leaves in order
    pub fn get_mantle_tree(&self) -> Result<Vec<String>> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("mantle", TREE_DEPTH)?;

        let nodes = merkle_nodes::table
            .filter(merkle_nodes::tree_id.eq(tree.tree_id))
            .filter(merkle_nodes::level.eq(0))
            .order(merkle_nodes::node_index.asc())
            .select(DbMerkleNode::as_select())
            .load::<DbMerkleNode>(&mut conn)
            .context("Failed to get mantle tree leaves")?;

        Ok(nodes.into_iter().map(|n| n.hash).collect())
    }

    /// Get Ethereum tree leaves in order
    pub fn get_ethereum_tree(&self) -> Result<Vec<String>> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        let nodes = merkle_nodes::table
            .filter(merkle_nodes::tree_id.eq(tree.tree_id))
            .filter(merkle_nodes::level.eq(0))
            .order(merkle_nodes::node_index.asc())
            .select(DbMerkleNode::as_select())
            .load::<DbMerkleNode>(&mut conn)
            .context("Failed to get ethereum tree leaves")?;

        Ok(nodes.into_iter().map(|n| n.hash).collect())
    }

    /// Get Mantle tree size (leaf count)
    pub fn get_mantle_tree_size(&self) -> Result<usize> {
        let tree = self.ensure_merkle_tree("mantle", TREE_DEPTH)?;

        Ok(tree.leaf_count as usize)
    }

    /// Get Ethereum tree size (leaf count)
    pub fn get_ethereum_tree_size(&self) -> Result<usize> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        Ok(tree.leaf_count as usize)
    }

    /// Add leaf to Mantle tree and increment counter
    pub fn add_to_mantle_tree(&self, _commitment: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("mantle", TREE_DEPTH)?;

        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    /// Add leaf to Ethereum tree and increment counter
    pub fn add_to_ethereum_tree(&self, _intent_id: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    /// Set Mantle node at specific level and index
    pub fn set_mantle_node(&self, level: usize, index: usize, hash: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("mantle", TREE_DEPTH)?;

        self.store_merkle_node(tree.tree_id, level as i32, index as i64, hash)?;
        Ok(())
    }

    /// Set Ethereum node at specific level and index
    pub fn set_ethereum_node(&self, level: usize, index: usize, hash: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        self.store_merkle_node(tree.tree_id, level as i32, index as i64, hash)?;
        Ok(())
    }

    /// Get Mantle node at specific level and index
    pub fn get_mantle_node(&self, level: usize, index: usize) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("mantle", TREE_DEPTH)?;

        let node = self.get_merkle_node(tree.tree_id, level as i32, index as i64)?;
        Ok(node.map(|n| n.hash))
    }

    /// Get Ethereum node at specific level and index
    pub fn get_ethereum_node(&self, level: usize, index: usize) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        let node = self.get_merkle_node(tree.tree_id, level as i32, index as i64)?;
        Ok(node.map(|n| n.hash))
    }

    /// Clear Mantle tree (reset leaf count and root)
    pub fn clear_mantle_tree(&self) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("mantle", TREE_DEPTH)?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree.tree_id)))
            .set((
                merkle_trees::leaf_count.eq(0),
                merkle_trees::root
                    .eq("0x0000000000000000000000000000000000000000000000000000000000000000"),
                merkle_trees::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to clear mantle tree")?;

        Ok(())
    }

    /// Clear Ethereum tree (reset leaf count and root)
    pub fn clear_ethereum_tree(&self) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree.tree_id)))
            .set((
                merkle_trees::leaf_count.eq(0),
                merkle_trees::root
                    .eq("0x0000000000000000000000000000000000000000000000000000000000000000"),
                merkle_trees::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to clear ethereum tree")?;

        Ok(())
    }

    /// Clear all Mantle nodes (for rebuilding)
    pub fn clear_mantle_nodes(&self) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("mantle", TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)
            .context("Failed to clear mantle nodes")?;

        Ok(())
    }

    pub fn clear_ethereum_nodes(&self) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn record_root(&self, chain: &str, root: &str) -> Result<()> {
        // let mut conn = self.get_connection()?;

        let tree = self
            .get_merkle_tree_by_name(chain)?
            .ok_or_else(|| anyhow::anyhow!("Tree {} not found", chain))?;

        self.update_merkle_root(tree.tree_id, root)?;

        Ok(())
    }

    pub fn get_last_synced_root_by_type(&self, sync_type: &str) -> Result<Option<String>> {
        let mut conn = self.get_connection()?;

        let result = root_syncs::table
            .filter(root_syncs::sync_type.eq(sync_type))
            .order(root_syncs::created_at.desc())
            .select(root_syncs::root)
            .first::<String>(&mut conn)
            .optional()
            .context("Failed to fetch last synced root by type")?;

        Ok(result)
    }

    pub fn get_latest_root(&self, chain: &str) -> Result<Option<String>> {
        let tree = self.get_merkle_tree_by_name(chain)?;
        Ok(tree.map(|t| t.root))
    }

    pub fn record_root_sync(&self, sync_type: &str, root: &str, tx_hash: &str) -> Result<()> {
        let event_data = serde_json::json!({
            "sync_type": sync_type,
            "root": root,
            "tx_hash": tx_hash,
        });

        let event_id = format!("root_sync_{}_{}", sync_type, chrono::Utc::now().timestamp());

        self.store_bridge_event(
            &event_id,
            None,
            "root_sync",
            event_data,
            0, // Chain ID 0 for cross-chain syncs
            0, // Block number not applicable
            tx_hash,
        )?;

        Ok(())
    }

    pub fn get_all_mantle_intents(&self) -> Result<Vec<MantleIntent>> {
        use crate::models::schema::bridge_events;

        let mut conn = self.get_connection()?;

        let events = bridge_events::table
            .filter(bridge_events::event_type.eq("intent_created"))
            .filter(bridge_events::chain_id.eq(5003)) // Mantle Sepolia chain ID
            .order((
                bridge_events::block_number.asc(),
                bridge_events::created_at.asc(),
            ))
            .select(bridge_events::all_columns)
            .load::<DbBridgeEvent>(&mut conn)?;

        let intents: Vec<MantleIntent> = events
            .into_iter()
            .filter_map(|e| {
                let commitment = e.event_data.get("commitment")?.as_str()?;
                Some(MantleIntent {
                    commitment: commitment.to_string(),
                    block_number: e.block_number as u64,
                    log_index: 0,
                })
            })
            .collect();

        Ok(intents)
    }

    pub fn get_all_mantle_fills(&self) -> Result<Vec<MantleFill>> {
        use crate::models::schema::bridge_events;

        let mut conn = self.get_connection()?;

        let events = bridge_events::table
            .filter(bridge_events::event_type.eq("intent_filled"))
            .filter(bridge_events::chain_id.eq(5003)) // Mantle Sepolia chain ID
            .order((
                bridge_events::block_number.asc(),
                bridge_events::created_at.asc(),
            ))
            .select(bridge_events::all_columns)
            .load::<DbBridgeEvent>(&mut conn)?;

        let fills: Vec<MantleFill> = events
            .into_iter()
            .filter_map(|e| {
                let intent_id = e.event_data.get("intent_id")?.as_str()?;
                let log_index = e
                    .event_data
                    .get("log_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                Some(MantleFill {
                    intent_id: intent_id.to_string(),
                    block_number: e.block_number as u64,
                    log_index,
                })
            })
            .collect();

        Ok(fills)
    }

    pub fn get_all_ethereum_intents(&self) -> Result<Vec<EthereumIntent>> {
        use crate::models::schema::bridge_events;

        let mut conn = self.get_connection()?;

        let events = bridge_events::table
            .filter(bridge_events::event_type.eq("intent_created"))
            .filter(bridge_events::chain_id.eq(11155111)) // Ethereum Sepolia chain ID
            .order((
                bridge_events::block_number.asc(),
                bridge_events::created_at.asc(),
            ))
            .select(bridge_events::all_columns)
            .load::<DbBridgeEvent>(&mut conn)?;

        let intents: Vec<EthereumIntent> = events
            .into_iter()
            .filter_map(|e| {
                let commitment = e.event_data.get("commitment")?.as_str()?;
                let log_index = e
                    .event_data
                    .get("log_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                Some(EthereumIntent {
                    commitment: commitment.to_string(),
                    block_number: e.block_number as u64,
                    log_index,
                })
            })
            .collect();

        Ok(intents)
    }

    pub fn get_all_ethereum_fills(&self) -> Result<Vec<EthereumFill>> {
        use crate::models::schema::bridge_events;

        let mut conn = self.get_connection()?;

        let events = bridge_events::table
            .filter(bridge_events::event_type.eq("intent_filled"))
            .filter(bridge_events::chain_id.eq(11155111)) // Ethereum Sepolia chain ID
            .order((
                bridge_events::block_number.asc(),
                bridge_events::created_at.asc(),
            ))
            .select(bridge_events::all_columns)
            .load::<DbBridgeEvent>(&mut conn)?;

        let fills: Vec<EthereumFill> = events
            .into_iter()
            .filter_map(|e| {
                let intent_id = e.event_data.get("intent_id")?.as_str()?;
                Some(EthereumFill {
                    intent_id: intent_id.to_string(),
                    block_number: e.block_number as u64,
                    log_index: 0, // Add log_index to bridge_events if needed
                })
            })
            .collect();

        Ok(fills)
    }

    pub fn get_all_ethereum_commitments(&self) -> Result<Vec<String>> {
        let mut conn = self.get_connection()?;

        let rows = ethereum_sepolia_intent_created::table
            .select(DbEthereumIntentCreated::as_select())
            .order((
                ethereum_sepolia_intent_created::block_number.asc(),
                ethereum_sepolia_intent_created::log_index.asc(),
            ))
            .load::<DbEthereumIntentCreated>(&mut conn)
            .context("Failed to load ethereum intent created events")?;

        let commitments = rows
            .into_iter()
            .filter_map(|row| row.event_data.get("commitment")?.as_str().map(String::from))
            .collect();

        Ok(commitments)
    }

    pub fn get_all_mantle_commitments(&self) -> Result<Vec<String>> {
        let mut conn = self.get_connection()?;

        let rows = mantle_sepolia_intent_created::table
            .select(DbMantleIntentCreated::as_select())
            .order((
                mantle_sepolia_intent_created::block_number.asc(),
                mantle_sepolia_intent_created::log_index.asc(),
            ))
            .load::<DbMantleIntentCreated>(&mut conn)
            .context("Failed to load mantle intent created events")?;

        let commitments = rows
            .into_iter()
            .filter_map(|row| row.event_data.get("commitment")?.as_str().map(String::from))
            .collect();

        Ok(commitments)
    }

    // ==================== Statistics ====================

    pub fn get_bridge_stats(&self) -> Result<BridgeStats> {
        let mut conn = self.get_connection()?;

        let total_intents: i64 = intents::table.count().get_result(&mut conn)?;

        let pending_intents: i64 = intents::table
            .filter(intents::status.eq_any(vec!["created", "committed"]))
            .count()
            .get_result(&mut conn)?;

        let filled_intents: i64 = intents::table
            .filter(intents::status.eq("filled"))
            .count()
            .get_result(&mut conn)?;

        let completed_intents: i64 = intents::table
            .filter(intents::status.eq("completed"))
            .count()
            .get_result(&mut conn)?;

        let failed_intents: i64 = intents::table
            .filter(intents::status.eq("failed"))
            .count()
            .get_result(&mut conn)?;

        let refunded_intents: i64 = intents::table
            .filter(intents::status.eq("refunded"))
            .count()
            .get_result(&mut conn)?;

        let ethereum_to_mantle: i64 = intents::table
            .filter(intents::source_chain.eq("ethereum"))
            .filter(intents::dest_chain.eq("mantle"))
            .count()
            .get_result(&mut conn)?;

        let mantle_to_ethereum: i64 = intents::table
            .filter(intents::source_chain.eq("mantle"))
            .filter(intents::dest_chain.eq("ethereum"))
            .count()
            .get_result(&mut conn)?;

        let completed: Vec<DbIntent> = intents::table
            .filter(intents::status.eq("completed"))
            .select(DbIntent::as_select()) // ✅ Add this
            .load::<DbIntent>(&mut conn)?;

        let completed: Vec<Intent> = completed.into_iter().map(db_intent_to_model).collect();

        let mut total_volumes_u128 = HashMap::new();
        for intent in completed {
            let amount = intent.amount.parse::<u128>().unwrap_or(0);
            *total_volumes_u128
                .entry(intent.source_token)
                .or_insert(0u128) += amount;
        }

        let total_volume_by_token: HashMap<String, String> = total_volumes_u128
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();

        Ok(BridgeStats {
            total_intents: total_intents as u64,
            pending_intents: pending_intents as u64,
            filled_intents: filled_intents as u64,
            completed_intents: completed_intents as u64,
            failed_intents: failed_intents as u64,
            refunded_intents: refunded_intents as u64,
            ethereum_to_mantle: ethereum_to_mantle as u64,
            mantle_to_ethereum: mantle_to_ethereum as u64,
            total_volume_by_token,
        })
    }
}

fn parse_status(s: &str) -> IntentStatus {
    match s {
        "created" => IntentStatus::Created,
        "filled" => IntentStatus::Filled,
        "completed" => IntentStatus::Completed,
        "refunded" => IntentStatus::Refunded,
        "failed" => IntentStatus::Failed,
        _ => IntentStatus::Failed,
    }
}

fn db_intent_to_model(r: DbIntent) -> Intent {
    Intent {
        id: r.id,
        user_address: r.user_address,
        source_chain: r.source_chain,
        dest_chain: r.dest_chain,
        source_token: r.source_token,
        dest_token: r.dest_token,
        amount: r.amount,
        dest_amount: r.dest_amount,
        source_commitment: r.source_commitment,
        dest_fill_txid: r.dest_fill_txid,
        dest_registration_txid: r.dest_registration_txid,
        source_complete_txid: r.source_complete_txid,
        status: parse_status(&r.status),
        created_at: r.created_at,
        updated_at: r.updated_at,
        deadline: r.deadline as u64,
        refund_address: r.refund_address,
    }
}
