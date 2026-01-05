mod api;
mod model;
mod solver;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http::header, middleware::Logger, web};
use anyhow::{Context, Result};
use tokio::signal;
use tracing::{error, info, warn};

use crate::api::config::configure_routes;
use crate::{model::SolverConfig, solver::CrossChainSolver};

pub struct AppState {
    pub solver: Arc<CrossChainSolver>,
    pub start_time: std::time::Instant,
}

fn load_config() -> Result<SolverConfig> {
    Ok(SolverConfig {
        ethereum_rpc: std::env::var("ETHEREUM_WS_RPC").context("ETHEREUM_WS_RPC not set")?,
        mantle_rpc: std::env::var("MANTLE_WS_RPC").context("MANTLE_WS_RPC not set")?,
        solver_private_key: std::env::var("SOLVER_PRIVATE_KEY")
            .context("SOLVER_PRIVATE_KEY not set")?,
        ethereum_settlement: std::env::var("ETHEREUM_SETTLEMENT")
            .context("ETHEREUM_SETTLEMENT not set")?
            .parse()?,
        mantle_settlement: std::env::var("MANTLE_SETTLEMENT")
            .context("MANTLE_SETTLEMENT not set")?
            .parse()?,
        ethereum_intent_pool: std::env::var("ETHEREUM_INTENT_POOL")
            .context("ETHEREUM_INTENT_POOL not set")?
            .parse()?,
        mantle_intent_pool: std::env::var("MANTLE_INTENT_POOL")
            .context("MANTLE_INTENT_POOL not set")?
            .parse()?,
        solver_address: std::env::var("SOLVER_ADDRESS")
            .context("SOLVER_ADDRESS not set")?
            .parse()?,
        ..Default::default()
    })
}

fn mask_url(url: &str) -> String {
    if let Some(pos) = url.rfind('/') {
        format!("{}/***/", &url[..pos])
    } else {
        "***".to_string()
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "solver=info,actix_web=info".into()),
        )
        .init();

    info!("🚀 Starting Private Bridge Solver v1.0.0");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    info!("📋 Loading configuration");
    let config = load_config().context("Failed to load configuration")?;

    info!("📡 Network Configuration:");
    info!("   • Ethereum RPC: {}", mask_url(&config.ethereum_rpc));
    info!("   • Mantle RPC: {}", mask_url(&config.mantle_rpc));
    info!("   • Solver Address: {:?}", config.solver_address);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    info!("🔧 Initializing solver");
    let solver = Arc::new(
        CrossChainSolver::new(config.clone())
            .await
            .context("Failed to initialize solver")?,
    );

    info!("✅ Solver initialized successfully");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 Solver Configuration:");
    info!("   • Max concurrent fills: {}", config.max_concurrent_fills);
    info!("   • Min profit threshold: {} bps", config.min_profit_bps);
    info!(
        "   • Source confirmations: {}",
        config.source_confirmations_required
    );
    info!("   • Max gas price: {} gwei", config.max_gas_price_gwei);
    info!(
        "   • Health check interval: {}s",
        config.health_check_interval_secs
    );
    info!(
        "   • Balance check interval: {}s",
        config.balance_check_interval_secs
    );
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("💰 Supported Tokens:");
    info!("   • ETH (Native Ethereum)");
    info!("   • WETH (Wrapped Ether)");
    info!("   • USDC (USD Coin)");
    info!("   • USDT (Tether)");
    info!("   • MNT (Native Mantle)");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let app_state = web::Data::new(AppState {
        solver: solver.clone(),
        start_time: std::time::Instant::now(),
    });

    info!("🏃 Starting solver main loop");
    let solver_handle = tokio::spawn({
        let solver = solver.clone();
        async move {
            if let Err(e) = solver.run().await {
                error!("❌ Solver error: {}", e);
            }
        }
    });

    let host = std::env::var("HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("HTTP_PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .context("Invalid HTTP_PORT")?;

    info!("🌐 Starting HTTP server on {}:{}", host, port);

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(
                &std::env::var("CORS_ORIGIN")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            )
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![header::CONTENT_TYPE, header::ACCEPT])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .configure(configure_routes)
            .wrap(cors)
            .wrap(Logger::default())
    })
    .bind((host.as_str(), port))
    .context("Failed to bind HTTP server")?
    .run();

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🌐 HTTP Endpoints:");
    info!("   • Health:    http://{}:{}/health", host, port);
    info!("   • Metrics:   http://{}:{}/metrics", host, port);
    info!("   • Status:    http://{}:{}/status", host, port);
    info!("   • Readiness: http://{}:{}/ready", host, port);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("✅ All services started successfully");
    info!("👀 Monitoring for intents...");
    info!("   Press Ctrl+C to shutdown gracefully");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    tokio::select! {
        result = server => {
            error!("❌ HTTP server stopped: {:?}", result);
        },
        _ = solver_handle => {
            error!("❌ Solver stopped unexpectedly");
        },
        _ = signal::ctrl_c() => {
            warn!("⚠️ Received shutdown signal (Ctrl+C)");
            info!("🛑 Initiating graceful shutdown...");
        }
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("✅ Solver stopped gracefully");
    info!("👋 Goodbye!");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
