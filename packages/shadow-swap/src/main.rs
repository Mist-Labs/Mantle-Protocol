mod api;
mod config;
mod database;
mod encryption;
mod ethereum;
mod intent_workers;
mod mantle;
mod merkle_manager;
mod models;
mod pricefeed;
mod relay_coordinator;
mod root_sync_coordinator;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http::header, middleware::Logger, web};
use anyhow::{Context, Result};
use tokio::task;
use tracing::{error, info};

use crate::{
    database::database::Database,
    intent_workers::{
        intent_registration_worker::IntentRegistrationWorker,
        intent_settlement_worker::IntentSettlementWorker,
    },
    merkle_manager::merkle_manager::MerkleTreeManager,
    models::model::BridgeConfig,
    pricefeed::pricefeed::PriceFeedManager,
    relay_coordinator::model::{BridgeCoordinator, EthereumRelayer, MantleRelayer},
    root_sync_coordinator::root_sync_coordinator::RootSyncCoordinator,
};

pub struct AppState {
    pub database: Arc<Database>,
    pub config: BridgeConfig,
    pub ethereum_relayer: Arc<EthereumRelayer>,
    pub mantle_relayer: Arc<MantleRelayer>,
    pub bridge_coordinator: Arc<BridgeCoordinator>,
    pub merkle_manager: Arc<MerkleTreeManager>,
    pub price_feed: Arc<PriceFeedManager>,
    pub root_sync_coordinator: Arc<RootSyncCoordinator>,
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mantle_bridge=info,actix_web=info".into()),
        )
        .init();

    info!("🚀 Starting Mantle Bridge Relayer");

    let config = BridgeConfig::from_env()
        .or_else(|_| BridgeConfig::from_file("config.toml".into()))
        .context("Failed to load configuration")?;

    let database = Arc::new(Database::from_env().context("Failed to initialize database")?);

    info!("📊 Running database migrations");
    Database::run_migrations(&database.pool).context("Failed to run migrations")?;

    info!("💱 Initializing price feeds");
    let price_feed = Arc::new(PriceFeedManager::new());

    info!("📈 Starting ETH<->MNT price feeds");
    price_feed.init_all_feeds().await;

    info!("🔗 Initializing Ethereum relayer");
    let ethereum_relayer = Arc::new(
        EthereumRelayer::new(config.ethereum.clone(), database.clone())
            .await
            .context("Failed to initialize Ethereum relayer")?,
    );

    info!("🔗 Initializing Mantle relayer");
    let mantle_relayer = Arc::new(
        MantleRelayer::new(config.mantle.clone(), database.clone())
            .await
            .context("Failed to initialize Mantle relayer")?,
    );

    info!("🌳 Initializing Merkle Tree Manager");
    let merkle_manager = Arc::new(MerkleTreeManager::new(
        mantle_relayer.clone(),
        ethereum_relayer.clone(),
        database.clone(),
        10,
    ));

    info!("🎯 Initializing bridge coordinator");
    let bridge_coordinator = Arc::new(BridgeCoordinator::new(
        ethereum_relayer.clone(),
        mantle_relayer.clone(),
        database.clone(),
        merkle_manager.clone(),
    ));

    info!("🔄 Initializing root sync coordinator");
    let root_sync_coordinator = Arc::new(RootSyncCoordinator::new(
        database.clone(),
        ethereum_relayer.clone(),
        mantle_relayer.clone(),
        10,
    ));

    info!("🔄 Initializing intent sync service");
    let intent_sync_service = Arc::new(intent_workers::event_sync::IntentSyncService::new(
        database.clone(),
        mantle_relayer.clone(),
        ethereum_relayer.clone(),
        merkle_manager.clone(),
    ));

    let app_state = web::Data::new(AppState {
        database: database.clone(),
        config: config.clone(),
        ethereum_relayer: ethereum_relayer.clone(),
        mantle_relayer: mantle_relayer.clone(),
        bridge_coordinator: bridge_coordinator.clone(),
        merkle_manager: merkle_manager.clone(),
        price_feed,
        root_sync_coordinator: root_sync_coordinator.clone(),
    });

    info!("🌳 Starting Merkle Tree Manager service");
    let tree_manager_handle = task::spawn({
        let manager = merkle_manager.clone();
        async move {
            if let Err(e) = manager.start().await {
                error!("❌ Merkle Tree Manager error: {}", e);
            }
        }
    });

    let should_sync_on_startup = std::env::var("SYNC_ON_STARTUP")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if should_sync_on_startup {
        info!("🔄 Performing initial sync on startup");

        let ethereum_from_block = std::env::var("ETHEREUM_SYNC_FROM_BLOCK")
            .unwrap_or_else(|_| "9995018".to_string())
            .parse::<u64>()
            .context("Invalid ETHEREUM_SYNC_FROM_BLOCK")?;

        let mantle_from_block = std::env::var("MANTLE_SYNC_FROM_BLOCK")
            .unwrap_or_else(|_| "33084800".to_string())
            .parse::<u64>()
            .context("Invalid MANTLE_SYNC_FROM_BLOCK")?;

        // --- Ethereum Sync ---
        info!("  Syncing Ethereum from block {}", ethereum_from_block);
        if let Err(e) = intent_sync_service
            .resync_ethereum_intents(ethereum_from_block, true)
            .await
        {
            error!("❌ Ethereum sync failed: {}", e);
        }

        // --- Mantle Sync ---
        info!("  Syncing Mantle from block {}", mantle_from_block);
        if let Err(e) = intent_sync_service
            .resync_mantle_intents(mantle_from_block, true)
            .await
        {
            error!("❌ Mantle sync failed: {}", e);
        }

        // --- Final Verification ---
        info!("🔍 Running final verification post-sync...");
        if let Err(e) = intent_sync_service.verify_sync_status().await {
            error!(
                "❌ Post-sync verification failed! Roots still do not match: {}",
                e
            );
        } else {
            info!("✅ Post-sync verification successful. All roots are consistent.");
        }
    }

    info!("⚙️  Starting bridge coordinator service");
    let coordinator_handle = task::spawn({
        let coordinator = bridge_coordinator.clone();
        async move {
            if let Err(e) = coordinator.start().await {
                error!("❌ Bridge coordinator error: {}", e);
            }
        }
    });

    info!("🔄 Starting root sync coordinator service");
    let root_sync_handle = task::spawn({
        let coordinator = root_sync_coordinator.clone();
        async move {
            coordinator.run().await;
        }
    });

    info!("📝 Starting intent registration worker");
    let registration_worker = Arc::new(IntentRegistrationWorker::new(
        database.clone(),
        mantle_relayer.clone(),
        ethereum_relayer.clone(),
        merkle_manager.clone(),
        root_sync_coordinator.clone(),
    ));

    let registration_handle = task::spawn({
        let worker = registration_worker.clone();
        async move {
            worker.run().await;
        }
    });

    info!("💰 Starting intent settlement worker");
    let settlement_worker = Arc::new(IntentSettlementWorker::new(
        database.clone(),
        mantle_relayer.clone(),
        ethereum_relayer.clone(),
        bridge_coordinator.clone(),
    ));

    let settlement_handle = task::spawn({
        let worker = settlement_worker.clone();
        async move {
            worker.run().await;
        }
    });

    let host = config.server.host.clone();
    let port = config.server.port;

    info!("🌐 Starting HTTP server on {}:{}", host, port);

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(
                &std::env::var("CORS_ORIGIN")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            )
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .configure(config::config_scope::configure)
            .wrap(cors)
            .wrap(Logger::default())
    })
    .bind((host.as_str(), port))
    .context("Failed to bind HTTP server")?
    .run();

    info!("✅ All services started successfully");

    tokio::select! {
        result = server => error!("HTTP server stopped: {:?}", result),
        _ = tree_manager_handle => error!("Merkle Tree Manager stopped unexpectedly"),
        _ = coordinator_handle => error!("Bridge coordinator stopped unexpectedly"),
        _ = root_sync_handle => error!("Root sync coordinator stopped unexpectedly"),
        _ = registration_handle => error!("Intent registration worker stopped unexpectedly"),
        _ = settlement_handle => error!("Intent settlement worker stopped unexpectedly"),
    }

    Ok(())
}
