use actix_web::{HttpResponse, Responder, get, web};
use serde_json::json;

use crate::{AppState, model::MetricsResponse};

#[get("/health")]
pub async fn health_check(data: web::Data<AppState>) -> impl Responder {
    let metric = data.solver.get_metrics().await;

    let status = if metric.last_error.is_some() {
        "degraded"
    } else {
        "healthy"
    };

    HttpResponse::Ok().json(json!({
        "status": status,
        "timestamp": chrono::Utc::now().timestamp(),
        "version": "1.0.0",
        "uptime_secs": data.start_time.elapsed().as_secs(),
        "active_fills": metric.active_fills_count,
        "last_error": metric.last_error,
    }))
}

#[get("/metrics")]
pub async fn metrics(data: web::Data<AppState>) -> impl Responder {
    let metrics = data.solver.get_metrics().await;

    let response = MetricsResponse {
        total_intents_evaluated: metrics.total_intents_evaluated,
        total_fills_attempted: metrics.total_fills_attempted,
        successful_fills: metrics.successful_fills,
        failed_fills: metrics.failed_fills,
        active_fills_count: metrics.active_fills_count,
        average_fill_time_secs: metrics.average_fill_time_secs,
        capital_deployed: metrics
            .capital_deployed
            .iter()
            .map(|(k, v)| (format!("{:?}", k), v.to_string()))
            .collect(),
        capital_available: metrics
            .capital_available
            .iter()
            .map(|((token, chain), amount)| (format!("{:?}-{}", token, chain), amount.to_string()))
            .collect(),
        total_profit_earned: metrics
            .total_profit_earned
            .iter()
            .map(|(k, v)| (format!("{:?}", k), v.to_string()))
            .collect(),
        last_error: metrics.last_error,
    };

    HttpResponse::Ok().json(response)
}

#[get("/status")]
pub async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let metric = data.solver.get_metrics().await;
    let config = &data.solver.config;

    HttpResponse::Ok().json(json!({
        "solver_address": format!("{:?}", config.solver_address),
        "ethereum_chain_id": config.ethereum_chain_id,
        "mantle_chain_id": config.mantle_chain_id,
        "max_concurrent_fills": config.max_concurrent_fills,
        "min_profit_bps": config.min_profit_bps,
        "uptime_secs": data.start_time.elapsed().as_secs(),
        "metrics": {
            "total_intents_evaluated": metric.total_intents_evaluated,
            "successful_fills": metric.successful_fills,
            "active_fills": metric.active_fills_count,
        },
    }))
}

#[get("/ready")]
pub async fn ready(data: web::Data<AppState>) -> impl Responder {
    let metric = data.solver.get_metrics().await;

    // Consider ready if no critical errors and can process fills
    if metric.last_error.is_none() || metric.successful_fills > 0 {
        HttpResponse::Ok().json(json!({"ready": true}))
    } else {
        HttpResponse::ServiceUnavailable().json(json!({
            "ready": false,
            "reason": metric.last_error
        }))
    }
}
