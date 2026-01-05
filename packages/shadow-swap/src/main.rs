mod api;
mod config;
mod database;
mod ethereum;
mod mantle;
mod merkle_manager;
mod models;
mod pricefeed;
mod relay_coordinator;
mod root_sync_coordinator;
mod encryption;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http::header, middleware::Logger, web};
use anyhow::{Context, Result};
use tokio::task;
use tracing::{error, info};

use crate::{
    database::database::Database,
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
        20,
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
        180,
    ));

    let app_state = web::Data::new(AppState {
        database,
        config: config.clone(),
        ethereum_relayer,
        mantle_relayer,
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
    }

    Ok(())
}
