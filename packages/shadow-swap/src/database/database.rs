use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenv::dotenv;
use serde_json::Value;
use tracing::{error, info, warn};

use crate::database::model::{
    BridgeStats, DbBridgeEvent, DbChainTransaction, DbEthereumIntentCreated, DbMantleIntentCreated,
    DbMerkleNode, DbMerkleTree, NewBridgeEvent, NewChainTransaction, NewMerkleNode, NewMerkleTree,
    NewRootSync,
};

use crate::models::model::{
    EthereumFill, EthereumIntent, IntentCreatedEvent, MantleFill, MantleIntent,
};
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
            .unwrap_or(20);

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
            solver_address: intent.solver_address.as_deref(),
            block_number: intent.block_number,
            log_index: intent.log_index,
        };

        diesel::insert_into(intents::table)
            .values(&new_intent)
            .on_conflict(intents::id)
            .do_nothing()
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
                solver_address: intent.solver_address.as_deref(),
                block_number: intent.block_number,
                log_index: intent.log_index,
            };

            diesel::insert_into(intents::table)
                .values(&new_intent)
                .on_conflict(intents::id)
                .do_nothing()
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

    ///     EVENT SYNC
    pub fn upsert_intent_from_event(
        &self,
        event: &IntentCreatedEvent,
        src_chain: &str,
    ) -> Result<()> {
        use crate::models::schema::intents::dsl::*;
        let mut conn = self.get_connection()?;

        let default_deadline = chrono::Utc::now().timestamp() + 3600;

        let new_intent = NewIntent {
            id: &event.intent_id,
            user_address: "0x0000000000000000000000000000000000000000",
            source_chain: src_chain,
            dest_chain: &event.dest_chain.to_string(),
            source_token: &event.source_token,
            dest_token: &event.dest_token,
            amount: &event.source_amount,
            dest_amount: &event.dest_amount,
            source_commitment: Some(&event.commitment),
            dest_fill_txid: None,
            dest_registration_txid: None,
            source_complete_txid: None,
            status: "committed",
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: event
                .deadline
                .filter(|d| *d > 0)
                .map(|d| d as i64)
                .unwrap_or(default_deadline),
            refund_address: None,
            solver_address: None,
            block_number: event.block_number.map(|b| b as i64),
            log_index: event.log_index.map(|i| i as i32),
        };

        let inserted: Option<DbIntent> = diesel::insert_into(intents)
            .values(&new_intent)
            .on_conflict(id)
            .do_nothing()
            .returning(DbIntent::as_select())
            .get_result(&mut conn)
            .optional()?;

        if inserted.is_some() {
            info!(
                "✅ Inserted intent {} from chain event",
                &event.intent_id[..10]
            );
        } else {
            info!(
                "ℹ️ Intent {} already exists, skipping",
                &event.intent_id[..10]
            );
        }

        Ok(())
    }

    pub fn clear_all_intents_for_chain(&self, chain_name: &str) -> Result<()> {
        use crate::models::schema::intents::dsl::*;
        let mut conn = self.get_connection()?;

        diesel::delete(intents.filter(source_chain.eq(chain_name))).execute(&mut conn)?;

        info!("🗑️  Cleared all intents for chain {}", chain_name);

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

    pub fn get_intents_by_status(&self, status: IntentStatus) -> Result<Vec<Intent>> {
        let mut conn = self.get_connection()?;

        let results = intents::table
            .filter(intents::status.eq(status.as_str()))
            .order(intents::created_at.asc())
            .select(DbIntent::as_select())
            .load::<DbIntent>(&mut conn)
            .context("Failed to get intents by status")?;

        Ok(results.into_iter().map(db_intent_to_model).collect())
    }

    pub fn get_pending_intents(&self) -> Result<Vec<Intent>> {
        let mut conn = self.get_connection()?;

        let results = intents::table
            .filter(intents::status.eq_any(vec!["created", "committed", "filled", "solver_paid"]))
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
        let normalized_id = intent_id.to_lowercase();

        if normalized_id.len() < 66 {
            warn!(
                "⚠️ Storing privacy params with a short ID: {} (length: {}). This may not match indexer records!",
                normalized_id,
                normalized_id.len()
            );
        }

        let new_params = NewIntentPrivacyParams {
            intent_id: &normalized_id,
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
            .context("Failed to store privacy params")?;

        info!("✅ Privacy params stored for intent: {}", normalized_id);
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

        diesel::update(intents::table.filter(intents::id.eq(&intent.id)))
            .set((
                intents::status.eq(intent.status.as_str()),
                intents::solver_address.eq(intent.solver_address.as_deref()),
                intents::dest_fill_txid.eq(intent.dest_fill_txid.as_deref()),
                intents::source_complete_txid.eq(intent.source_complete_txid.as_deref()),
                intents::dest_registration_txid.eq(intent.dest_registration_txid.as_deref()),
                intents::source_commitment.eq(intent.source_commitment.as_deref()),
                intents::updated_at.eq(intent.updated_at),
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
            "ethereum" => 11155111,
            "mantle" => 5003,
            _ => 0,
        };

        let event_data = serde_json::json!({
            "intent_id": intent_id,
            "event_type": event_type,
            "chain": chain,
            "block_number": block_number,
        });

        let event_id = format!("{}_{}_{}_{}", event_type, chain, tx_hash, block_number);

        self.store_bridge_event(
            &event_id,
            Some(intent_id),
            event_type,
            event_data,
            chain_id,
            block_number as i64,
            tx_hash,
        )
    }

    pub fn update_dest_registration_txid(&self, intent_id: &str, txid: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(intents::table.filter(intents::id.eq(intent_id)))
            .set((
                intents::dest_registration_txid.eq(txid),
                intents::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update dest registration txid")?;

        Ok(())
    }

    pub fn update_source_settlement_txid(&self, intent_id: &str, txid: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(intents::table.filter(intents::id.eq(intent_id)))
            .set((
                intents::source_settlement_txid.eq(txid),
                intents::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update source settlement txid")?;

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

    // ==================== SOLVER-RELATED OPERATIONS ====================
    pub fn get_intent_solver(&self, intent_id: &str) -> Result<Option<String>> {
        let mut conn = self.get_connection()?;

        let solver_address = intents::table
            .filter(intents::id.eq(intent_id))
            .select(intents::solver_address)
            .first::<Option<String>>(&mut conn)
            .optional()
            .context("Failed to get solver address")?;

        Ok(solver_address.flatten())
    }

    pub fn update_intent_with_solver(
        &self,
        intent_id: &str,
        solver: &str,
        new_status: IntentStatus,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(intents::table.filter(intents::id.eq(intent_id)))
            .set((
                intents::solver_address.eq(Some(solver)),
                intents::status.eq(new_status.as_str()),
                intents::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update intent with solver")?;

        Ok(())
    }

    pub fn update_solver_address(&self, intent_id: &str, solver_address: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(intents::table.filter(intents::id.eq(intent_id)))
            .set((
                intents::solver_address.eq(solver_address),
                intents::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update solver address")?;

        Ok(())
    }

    pub fn get_intents_by_solver(&self, solver_address: &str, limit: usize) -> Result<Vec<Intent>> {
        let mut conn = self.get_connection()?;

        let results = intents::table
            .filter(intents::solver_address.eq(solver_address))
            .order(intents::created_at.desc())
            .limit(limit as i64)
            .select(DbIntent::as_select())
            .load::<DbIntent>(&mut conn)
            .context("Failed to get intents by solver")?;

        Ok(results.into_iter().map(db_intent_to_model).collect())
    }

    pub fn get_solver_stats(&self, solver_address: &str) -> Result<(i64, f64)> {
        let mut conn = self.get_connection()?;

        // Count total intents filled by this solver
        let total_filled = intents::table
            .filter(intents::solver_address.eq(solver_address))
            .filter(intents::status.eq_any(vec!["filled", "completed", "solver_paid"]))
            .count()
            .get_result::<i64>(&mut conn)
            .context("Failed to count solver intents")?;

        // Calculate total volume (simplified - you may want to handle decimals better)
        let intents_list = self.get_intents_by_solver(solver_address, 10000)?;
        let total_volume: f64 = intents_list
            .iter()
            .filter_map(|intent| intent.amount.parse::<f64>().ok())
            .sum();

        Ok((total_filled, total_volume))
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
        event_data: Value,
        chain_id: i32,
        block_number: i64,
        transaction_hash: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;

        let new_event = NewBridgeEvent {
            event_id,
            intent_id,
            event_type,
            event_data,
            chain_id,
            block_number,
            transaction_hash,
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        diesel::insert_into(bridge_events::table)
            .values(&new_event)
            .on_conflict(bridge_events::event_id)
            .do_nothing() // Idempotency: if event_id exists, skip
            .execute(&mut conn)
            .map_err(|e| {
                error!("🔴 DB INSERT FAILED: event_id={}, error={}", event_id, e);
                e
            })
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

    pub fn insert_root_sync(&self, sync_type: &str, root: &str, tx_hash: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        let new_sync = NewRootSync {
            sync_type,
            root,
            tx_hash,
            created_at: Utc::now(),
        };

        diesel::insert_into(root_syncs::table)
            .values(&new_sync)
            .execute(&mut conn)
            .context("Failed to insert root sync")?;

        Ok(())
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

    pub fn update_merkle_root_by_name(&self, tree_name: &str, root: &str) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_name.eq(tree_name)))
            .set((
                merkle_trees::root.eq(root),
                merkle_trees::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to update merkle root by name")?;

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

    pub fn get_mantle_commitment_tree_size(&self) -> Result<usize> {
        let tree = self.ensure_merkle_tree("mantle_commitments", TREE_DEPTH)?;

        Ok(tree.leaf_count as usize)
    }

    pub fn add_to_ethereum_commitment_tree(&self, _commitment: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;

        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    pub fn add_to_mantle_commitment_tree(&self, _commitment: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("mantle_commitments", TREE_DEPTH)?;

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

    pub fn set_mantle_commitment_node(&self, level: usize, index: usize, hash: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("mantle_commitments", TREE_DEPTH)?;

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

    pub fn get_mantle_commitment_node(&self, level: usize, index: usize) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("mantle_commitments", TREE_DEPTH)?;

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

    pub fn clear_mantle_commitment_nodes(&self) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("mantle_commitments", TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)
            .context("Failed to clear mantle commitment nodes")?;

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

    pub fn clear_mantle_commitment_tree(&self) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("mantle_commitments", TREE_DEPTH)?;

        self.clear_mantle_commitment_nodes()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree.tree_id)))
            .set(merkle_trees::leaf_count.eq(0))
            .execute(&mut conn)
            .context("Failed to reset mantle commitment tree leaf count")?;

        Ok(())
    }

    pub fn get_ethereum_commitment_tree(&self) -> Result<Vec<String>> {
        let _tree = self.ensure_merkle_tree("ethereum_commitments", TREE_DEPTH)?;
        let commitments = self.get_all_ethereum_commitments()?;

        Ok(commitments)
    }

    pub fn get_mantle_commitment_tree(&self) -> Result<Vec<String>> {
        let _tree = self.ensure_merkle_tree("mantle_commitments", TREE_DEPTH)?;
        let commitments = self.get_all_mantle_commitments()?;

        Ok(commitments)
    }

    pub fn get_ethereum_intent_tree_size(&self) -> Result<usize> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;
        Ok(tree.leaf_count as usize)
    }

    pub fn get_ethereum_fill_tree_size(&self) -> Result<usize> {
        let tree = self.ensure_merkle_tree("ethereum_fills", TREE_DEPTH)?;
        Ok(tree.leaf_count as usize)
    }

    pub fn add_to_ethereum_intent_tree(&self, _commitment: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;
        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    pub fn add_to_ethereum_fill_tree(&self, _intent_id: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_fills", TREE_DEPTH)?;
        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    pub fn set_ethereum_intent_node(&self, level: usize, index: usize, hash: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;
        self.store_merkle_node(tree.tree_id, level as i32, index as i64, hash)?;
        Ok(())
    }

    pub fn set_ethereum_fill_node(&self, level: usize, index: usize, hash: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_fills", TREE_DEPTH)?;
        self.store_merkle_node(tree.tree_id, level as i32, index as i64, hash)?;
        Ok(())
    }

    pub fn get_ethereum_intent_node(&self, level: usize, index: usize) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;
        let node = self.get_merkle_node(tree.tree_id, level as i32, index as i64)?;
        Ok(node.map(|n| n.hash))
    }

    pub fn get_ethereum_fill_node(&self, level: usize, index: usize) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("ethereum_fills", TREE_DEPTH)?;
        let node = self.get_merkle_node(tree.tree_id, level as i32, index as i64)?;
        Ok(node.map(|n| n.hash))
    }

    pub fn get_ethereum_intent_tree(&self) -> Result<Vec<String>> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

        let nodes = merkle_nodes::table
            .filter(merkle_nodes::tree_id.eq(tree.tree_id))
            .filter(merkle_nodes::level.eq(0))
            .order(merkle_nodes::node_index.asc())
            .select(DbMerkleNode::as_select())
            .load::<DbMerkleNode>(&mut conn)
            .context("Failed to get ethereum intent tree leaves")?;

        Ok(nodes.into_iter().map(|n| n.hash).collect())
    }

    pub fn get_ethereum_fill_tree(&self) -> Result<Vec<String>> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;
        let tree = self.ensure_merkle_tree("ethereum_fills", TREE_DEPTH)?;

        let nodes = merkle_nodes::table
            .filter(merkle_nodes::tree_id.eq(tree.tree_id))
            .filter(merkle_nodes::level.eq(0))
            .order(merkle_nodes::node_index.asc())
            .select(DbMerkleNode::as_select())
            .load::<DbMerkleNode>(&mut conn)
            .context("Failed to get ethereum fill tree leaves")?;

        Ok(nodes.into_iter().map(|n| n.hash).collect())
    }

    pub fn clear_ethereum_intent_tree(&self) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

        self.clear_ethereum_intent_nodes()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree.tree_id)))
            .set(merkle_trees::leaf_count.eq(0))
            .execute(&mut conn)
            .context("Failed to reset ethereum intent tree leaf count")?;

        Ok(())
    }

    pub fn clear_ethereum_fill_tree(&self) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;
        let tree = self.ensure_merkle_tree("ethereum_fills", TREE_DEPTH)?;

        self.clear_ethereum_fill_nodes()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree.tree_id)))
            .set(merkle_trees::leaf_count.eq(0))
            .execute(&mut conn)
            .context("Failed to reset ethereum fill tree leaf count")?;

        Ok(())
    }

    pub fn clear_ethereum_intent_nodes(&self) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)
            .context("Failed to clear ethereum intent nodes")?;

        Ok(())
    }

    pub fn clear_ethereum_fill_nodes(&self) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;
        let tree = self.ensure_merkle_tree("ethereum_fills", TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)
            .context("Failed to clear ethereum fill nodes")?;

        Ok(())
    }

    pub fn clear_merkle_tree_completely(&self, tree_name: &str) -> Result<()> {
        use crate::models::schema::{merkle_nodes, merkle_trees};

        let mut conn = self.get_connection()?;

        // Use transaction for atomicity
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let tree = merkle_trees::table
                .filter(merkle_trees::tree_name.eq(tree_name))
                .select(DbMerkleTree::as_select())
                .first::<DbMerkleTree>(conn)
                .optional()
                .context("Failed to get tree")?;

            if let Some(tree) = tree {
                // Clear all nodes
                diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
                    .execute(conn)
                    .context("Failed to clear merkle nodes")?;

                // Reset tree metadata
                diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree.tree_id)))
                    .set((
                        merkle_trees::leaf_count.eq(0),
                        merkle_trees::root.eq(
                            "0x0000000000000000000000000000000000000000000000000000000000000000",
                        ),
                        merkle_trees::updated_at.eq(Utc::now()),
                    ))
                    .execute(conn)
                    .context("Failed to reset tree metadata")?;

                info!("🗑️  Cleared tree '{}' completely", tree_name);
            }

            Ok(())
        })?;

        Ok(())
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
    pub fn clear_merkle_nodes_by_tree(&self, target_tree_id: i32) -> Result<()> {
        use crate::models::schema::merkle_nodes;
        let mut conn = self.get_connection()?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(target_tree_id)))
            .execute(&mut conn)
            .context("Failed to clear merkle nodes for the specified tree")?;
        Ok(())
    }

    pub fn reset_leaf_count(&self, tree_id: i32) -> Result<()> {
        use crate::models::schema::merkle_trees;
        let mut conn = self.get_connection()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree_id)))
            .set(merkle_trees::leaf_count.eq(0))
            .execute(&mut conn)
            .context("Failed to reset leaf count")?;
        Ok(())
    }

    pub fn set_leaf_count(&self, tree_id: i32, count: i64) -> Result<()> {
        let mut conn = self.get_connection()?;

        diesel::update(merkle_trees::table.filter(merkle_trees::tree_id.eq(tree_id)))
            .set((
                merkle_trees::leaf_count.eq(count),
                merkle_trees::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .context("Failed to set leaf count")?;

        Ok(())
    }

    pub fn get_tree_size(&self, tree_name: &str) -> Result<usize> {
        let tree = self
            .get_merkle_tree_by_name(tree_name)?
            .ok_or_else(|| anyhow!("Tree '{}' not found", tree_name))?;

        Ok(tree.leaf_count as usize)
    }

    pub fn verify_tree_consistency(&self, tree_name: &str) -> Result<bool> {
        let tree = self
            .get_merkle_tree_by_name(tree_name)?
            .ok_or_else(|| anyhow!("Tree '{}' not found", tree_name))?;

        let chain = if tree_name.contains("mantle") {
            "mantle"
        } else {
            "ethereum"
        };

        let actual_commitments = self.get_all_commitments_for_chain(chain)?;
        let tree_leaf_count = tree.leaf_count as usize;

        info!(
            "🔍 Tree '{}': metadata says {} leaves, database has {} commitments",
            tree_name,
            tree_leaf_count,
            actual_commitments.len()
        );

        Ok(tree_leaf_count == actual_commitments.len())
    }

    // ==================== Merkle Tree Operations ====================

    /// Get Mantle tree leaves in order
    pub fn get_mantle_tree(&self) -> Result<Vec<String>> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("mantle_intents", TREE_DEPTH)?;

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

        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

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
        let tree = self.ensure_merkle_tree("mantle_intents", TREE_DEPTH)?;

        Ok(tree.leaf_count as usize)
    }

    /// Get Ethereum tree size (leaf count)
    pub fn get_ethereum_tree_size(&self) -> Result<usize> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

        Ok(tree.leaf_count as usize)
    }

    /// Add leaf to Mantle tree and increment counter
    pub fn add_to_mantle_tree(&self, _commitment: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("mantle_intents", TREE_DEPTH)?;

        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    /// Add leaf to Ethereum tree and increment counter
    pub fn add_to_ethereum_tree(&self, _intent_id: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

        self.increment_leaf_count(tree.tree_id, 1)?;
        Ok(())
    }

    /// Set Mantle node at specific level and index
    pub fn set_mantle_node(&self, level: usize, index: usize, hash: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("mantle_intents", TREE_DEPTH)?;

        self.store_merkle_node(tree.tree_id, level as i32, index as i64, hash)?;
        Ok(())
    }

    /// Set Ethereum node at specific level and index
    pub fn set_ethereum_node(&self, level: usize, index: usize, hash: &str) -> Result<()> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

        self.store_merkle_node(tree.tree_id, level as i32, index as i64, hash)?;
        Ok(())
    }

    /// Get Mantle node at specific level and index
    pub fn get_mantle_node(&self, level: usize, index: usize) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("mantle_intents", TREE_DEPTH)?;

        let node = self.get_merkle_node(tree.tree_id, level as i32, index as i64)?;
        Ok(node.map(|n| n.hash))
    }

    /// Get Ethereum node at specific level and index
    pub fn get_ethereum_node(&self, level: usize, index: usize) -> Result<Option<String>> {
        let tree = self.ensure_merkle_tree("ethereum_intents", TREE_DEPTH)?;

        let node = self.get_merkle_node(tree.tree_id, level as i32, index as i64)?;
        Ok(node.map(|n| n.hash))
    }

    /// Clear Mantle tree (reset leaf count and root)
    pub fn clear_mantle_tree(&self) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree("mantle_intents", TREE_DEPTH)?;

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
    pub fn clear_ethereum_tree(&self, tree_name: &str) -> Result<()> {
        use crate::models::schema::merkle_trees;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree(tree_name, TREE_DEPTH)?;

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

        let tree = self.ensure_merkle_tree("mantle_intents", TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)
            .context("Failed to clear mantle nodes")?;

        Ok(())
    }

    pub fn clear_ethereum_nodes(&self, tree_name: &str) -> Result<()> {
        use crate::models::schema::merkle_nodes;

        let mut conn = self.get_connection()?;

        let tree = self.ensure_merkle_tree(tree_name, TREE_DEPTH)?;

        diesel::delete(merkle_nodes::table.filter(merkle_nodes::tree_id.eq(tree.tree_id)))
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn record_root(&self, chain: &str, root: &str) -> Result<()> {
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

    pub fn get_all_mantle_fills(&self) -> Result<Vec<MantleFill>> {
        use crate::models::schema::bridge_events;

        let mut conn = self.get_connection()?;

        let events = bridge_events::table
            .filter(bridge_events::event_type.eq("intent_filled"))
            .filter(bridge_events::chain_id.eq(5003)) // Mantle Sepolia chain ID
            .order((
                bridge_events::block_number.asc(),
                bridge_events::created_at.asc(),
                bridge_events::id.asc(),
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

    pub fn get_all_ethereum_fills(&self) -> Result<Vec<EthereumFill>> {
        use crate::models::schema::bridge_events;

        let mut conn = self.get_connection()?;

        let events = bridge_events::table
            .filter(bridge_events::event_type.eq("intent_filled"))
            .filter(bridge_events::chain_id.eq(11155111)) // Ethereum Sepolia chain ID
            .order((
                bridge_events::block_number.asc(),
                bridge_events::created_at.asc(),
                bridge_events::id.asc(),
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
        use crate::models::schema::intents::dsl::*;
        let mut conn = self.get_connection()?;

        let rows: Vec<Option<String>> = intents
            .filter(
                source_chain
                    .eq("ethereum")
                    .and(source_commitment.is_not_null()),
            )
            .select(source_commitment)
            .order((
                block_number.asc().nulls_last(),
                log_index.asc().nulls_last(),
            ))
            .load(&mut conn)?;

        let commitments: Vec<String> = rows.into_iter().flatten().collect();

        if commitments.is_empty() {
            return Err(anyhow!(
                "🚨 No Ethereum commitments found – Merkle tree cannot be built"
            ));
        }

        Ok(commitments)
    }

    pub fn get_all_mantle_commitments(&self) -> Result<Vec<String>> {
        use crate::models::schema::intents::dsl::*;
        let mut conn = self.get_connection()?;

        let rows: Vec<Option<String>> = intents
            .filter(
                source_chain
                    .eq("mantle")
                    .and(source_commitment.is_not_null()),
            )
            .select(source_commitment)
            .order((
                block_number.asc().nulls_last(),
                log_index.asc().nulls_last(),
            ))
            .load(&mut conn)?;

        let commitments: Vec<String> = rows.into_iter().flatten().collect();

        if commitments.is_empty() {
            return Err(anyhow!(
                "🚨 No Mantle commitments found – Merkle tree cannot be built"
            ));
        }

        Ok(commitments)
    }

    pub fn get_all_commitments_for_chain(&self, chain_name: &str) -> Result<Vec<String>> {
        use crate::models::schema::intents::dsl::*;
        let mut conn = self.get_connection()?;

        let commitments: Vec<String> = intents
            .filter(source_chain.eq(chain_name))
            .filter(source_commitment.is_not_null())
            .filter(block_number.is_not_null())
            .filter(log_index.is_not_null())
            .order((block_number.asc(), log_index.asc()))
            .select(source_commitment)
            .load::<Option<String>>(&mut conn)
            .context("Failed to load commitments column from intents table")?
            .into_iter()
            .flatten()
            .collect();

        info!(
            "📊 Loaded {} commitments for chain '{}'",
            commitments.len(),
            chain_name
        );

        Ok(commitments)
    }

    pub fn get_commitments_for_tree(&self, chain_name: &str, limit: i64) -> Result<Vec<String>> {
        use crate::models::schema::intents::dsl::*;
        let mut conn = self.get_connection()?;

        let commitments: Vec<String> = intents
            .filter(source_chain.eq(chain_name))
            .filter(source_commitment.is_not_null())
            .filter(block_number.is_not_null()) // ✅ CRITICAL
            .filter(log_index.is_not_null()) // ✅ CRITICAL
            .order((block_number.asc(), log_index.asc()))
            .limit(limit) // Only take first N commitments
            .select(source_commitment)
            .load::<Option<String>>(&mut conn)?
            .into_iter()
            .flatten()
            .collect();

        info!(
            "📊 Loaded {} commitments (limit: {}) for chain '{}'",
            commitments.len(),
            limit,
            chain_name
        );

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
        "registered" => IntentStatus::Registered,
        "committed" => IntentStatus::Committed,
        "pending" => IntentStatus::Pending,
        "filled" => IntentStatus::Filled,
        "user_claimed" => IntentStatus::UserClaimed,
        "solver_paid" => IntentStatus::SolverPaid,
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
        solver_address: r.solver_address,
        block_number: r.block_number,
        log_index: r.log_index,
    }
}
