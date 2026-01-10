use std::collections::HashMap;

use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use chrono::Utc;
use serde_json::json;
use tracing::{debug, error, info, warn};

use crate::{
    AppState,
    api::{
        helper::{
            handle_intent_created_event, handle_intent_filled_event, handle_intent_refunded_event,
            handle_intent_registered_event, handle_intent_settled_event, handle_root_synced_event,
            handle_withdrawal_claimed_event, validate_hmac,
        },
        model::{
            AllPricesResponse, ConvertRequest, ConvertResponse, IndexerEventRequest,
            IndexerEventResponse, InitiateBridgeRequest, InitiateBridgeResponse,
            IntentStatusResponse, PriceRequest, PriceResponse, PriceSourceInfo, StatsResponse,
        },
    },
    models::model::TokenType,
};

// ============================================================================
// BRIDGE OPERATIONS
// ============================================================================
// secret and nullifier should be encrypted on frontend before sending to backend
#[post("/bridge/initiate")]
pub async fn initiate_bridge(
    req: HttpRequest,
    body: web::Bytes,
    app_state: web::Data<AppState>,
) -> impl Responder {
    // HMAC validation
    if let Err(response) = validate_hmac(&req, &body, &app_state) {
        return response;
    }
    // Parse request body
    let request: InitiateBridgeRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return HttpResponse::BadRequest().json(InitiateBridgeResponse {
                success: false,
                intent_id: String::new(),
                commitment: String::new(),
                message: "Invalid request body".to_string(),
                error: Some(e.to_string()),
            });
        }
    };

    let intent_id = request.intent_id.to_lowercase();
    if !intent_id.starts_with("0x") || intent_id.len() != 66 {
        return HttpResponse::BadRequest().json(InitiateBridgeResponse {
            success: false,
            intent_id: intent_id.clone(),
            commitment: String::new(),
            message: "Invalid intent_id format".to_string(),
            error: Some("intent_id must be a 32-byte hex string (0x...)".to_string()),
        });
    }

    if !request.commitment.starts_with("0x") || request.commitment.len() != 66 {
        return HttpResponse::BadRequest().json(InitiateBridgeResponse {
            success: false,
            intent_id: String::new(),
            commitment: String::new(),
            message: "Invalid commitment format".to_string(),
            error: Some("Commitment must be 32-byte hex string".to_string()),
        });
    }

    if !request.encrypted_secret.starts_with("0x") {
        return HttpResponse::BadRequest().json(InitiateBridgeResponse {
            success: false,
            intent_id: String::new(),
            commitment: String::new(),
            message: "Invalid secret format".to_string(),
            error: Some("Secret must be 32-byte hex string".to_string()),
        });
    }

    if !request.encrypted_nullifier.starts_with("0x") {
        return HttpResponse::BadRequest().json(InitiateBridgeResponse {
            success: false,
            intent_id: String::new(),
            commitment: String::new(),
            message: "Invalid nullifier format".to_string(),
            error: Some("Nullifier must be 32-byte hex string".to_string()),
        });
    }

    if !request.claim_auth.starts_with("0x") || request.claim_auth.len() != 132 {
        return HttpResponse::BadRequest().json(InitiateBridgeResponse {
            success: false,
            intent_id: String::new(),
            commitment: String::new(),
            message: "Invalid claim_auth format".to_string(),
            error: Some("Claim authorization must be 65-byte hex signature".to_string()),
        });
    }

    info!(
        "🌉 Initiating bridge | Direction: {} -> {} | Token: {} | Amount: {}",
        request.source_chain, request.dest_chain, request.source_token, request.amount
    );

    if !matches!(
        (request.source_chain.as_str(), request.dest_chain.as_str()),
        ("ethereum", "mantle") | ("mantle", "ethereum")
    ) {
        return HttpResponse::BadRequest().json(InitiateBridgeResponse {
            success: false,
            intent_id: String::new(),
            commitment: String::new(),
            message: "Invalid chain pair".to_string(),
            error: Some("Must be ethereum<->mantle".to_string()),
        });
    }

    let _token_type = match TokenType::from_address(&request.source_token) {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::BadRequest().json(InitiateBridgeResponse {
                success: false,
                intent_id: String::new(),
                commitment: String::new(),
                message: "Unsupported token".to_string(),
                error: Some(e.to_string()),
            });
        }
    };

    if let Err(e) = app_state.database.store_intent_privacy_params(
        &intent_id,
        &request.commitment,
        &request.encrypted_secret,
        &request.encrypted_nullifier,
        &request.claim_auth,
        &request.recipient,
    ) {
        error!("Failed to store privacy params for {}: {}", intent_id, e);
        return HttpResponse::InternalServerError().json(InitiateBridgeResponse {
            success: false,
            intent_id: intent_id.clone(),
            commitment: String::new(),
            message: "Failed to store privacy parameters".to_string(),
            error: Some(e.to_string()),
        });
    }

    info!("✅ Bridge intent created: {}", intent_id);

    HttpResponse::Ok().json(InitiateBridgeResponse {
        success: true,
        intent_id: intent_id.clone(),
        commitment: request.commitment.clone(),
        message: format!(
            "Bridge intent created. Relayer will process on {}",
            request.source_chain
        ),
        error: None,
    })
}

#[get("/bridge/intent/{intent_id}")]
pub async fn get_intent_status(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let intent_id = path.into_inner();

    match app_state.database.get_intent_by_id(&intent_id) {
        Ok(Some(intent)) => {
            let privacy_params = app_state
                .database
                .get_intent_privacy_params(&intent_id)
                .ok();

            HttpResponse::Ok().json(IntentStatusResponse {
                intent_id: intent.id,
                status: intent.status.as_str().to_string(),
                source_chain: intent.source_chain,
                dest_chain: intent.dest_chain,
                source_token: intent.source_token,
                dest_token: intent.dest_token,
                amount: intent.amount,
                commitment: intent.source_commitment,
                dest_fill_txid: intent.dest_fill_txid,
                source_complete_txid: intent.source_complete_txid,
                deadline: intent.deadline,
                created_at: intent.created_at,
                updated_at: intent.updated_at,
                has_privacy: privacy_params.is_some(),
            })
        }
        Ok(None) => HttpResponse::NotFound().json(json!({
            "status": "error",
            "message": "Intent not found"
        })),
        Err(e) => {
            error!("Failed to get intent {}: {}", intent_id, e);
            HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": "Failed to retrieve intent"
            }))
        }
    }
}

#[get("/bridge/intents")]
pub async fn list_intents(
    app_state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let status_filter = query.get("status").map(|s| s.as_str());
    let chain_filter = query.get("chain").map(|s| s.as_str());
    let limit: usize = query
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50)
        .min(200);

    match app_state
        .database
        .list_intents(status_filter, chain_filter, limit)
    {
        Ok(intents) => HttpResponse::Ok().json(json!({
            "status": "success",
            "count": intents.len(),
            "data": intents
        })),
        Err(e) => {
            error!("Failed to list intents: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": "Failed to retrieve intents"
            }))
        }
    }
}

// ============================================================================
// INDEXER WEBHOOKS
// ============================================================================

#[post("/indexer/event")]
pub async fn indexer_event(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Bytes,
) -> impl Responder {
    // HMAC validation for security
    if let Ok(body_str) = std::str::from_utf8(&body) {
        debug!("📥 Received indexer event body: {}", body_str);
    }

    if let Err(response) = validate_hmac(&req, &body, &app_state) {
        return response;
    }

    let request: IndexerEventRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Invalid request format".to_string(),
                error: Some(e.to_string()),
            });
        }
    };

    info!(
        "📡 Indexer event: {} | Chain: {} | TxHash: {}",
        request.event_type, request.chain, request.transaction_hash
    );

    match request.event_type.as_str() {
        "intent_created" => handle_intent_created_event(&app_state, &request).await,
        "intent_filled" => handle_intent_filled_event(&app_state, &request).await,
        "intent_registered" => handle_intent_registered_event(&app_state, &request).await,
        "intent_settled" => handle_intent_settled_event(&app_state, &request).await,
        "intent_refunded" => handle_intent_refunded_event(&app_state, &request).await,
        "withdrawal_claimed" => handle_withdrawal_claimed_event(&app_state, &request).await,

        "root_synced" | "commitment_root_synced" | "fill_root_synced" => {
            handle_root_synced_event(&app_state, &request).await
        }

        _ => {
            warn!("Unknown event type: {}", request.event_type);
            HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: format!("Unknown event type: {}", request.event_type),
                error: None,
            })
        }
    }
}

// ============================================================================
// PRICE FEED ENDPOINTS
// ============================================================================

#[get("/price")]
pub async fn get_price(
    query: web::Query<PriceRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    info!(
        "📊 Price query: {} -> {}",
        query.from_symbol, query.to_symbol
    );

    let from_token = match TokenType::from_symbol(&query.from_symbol) {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid from_symbol: {}", e)
            }));
        }
    };

    let to_token = match TokenType::from_symbol(&query.to_symbol) {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid to_symbol: {}", e)
            }));
        }
    };

    match app_state
        .price_feed
        .get_exchange_rate(&from_token, &to_token)
        .await
    {
        Ok(rate) => {
            let converted_amount = query.amount.map(|amt| amt * rate);

            // Get price data for sources
            let all_prices = app_state.price_feed.get_all_prices().await;
            let pair_key = format!("{}-USD", from_token.symbol());
            let sources: Vec<PriceSourceInfo> = all_prices
                .get(&pair_key)
                .map(|pd| {
                    pd.sources
                        .iter()
                        .map(|s| PriceSourceInfo {
                            source: s.source.clone(),
                            price: s.price,
                        })
                        .collect()
                })
                .unwrap_or_default();

            let response = PriceResponse {
                from_symbol: from_token.symbol().to_string(),
                to_symbol: to_token.symbol().to_string(),
                rate,
                amount: query.amount,
                converted_amount,
                timestamp: Utc::now().timestamp(),
                sources,
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            warn!("Failed to get exchange rate: {}", e);
            HttpResponse::ServiceUnavailable().json(json!({
                "error": format!("Price data unavailable: {}", e)
            }))
        }
    }
}

#[get("/prices/all")]
pub async fn get_all_prices(app_state: web::Data<AppState>) -> impl Responder {
    info!("📊 Fetching all bridge token prices");

    let all_price_data = app_state.price_feed.get_all_prices().await;

    let mut prices = HashMap::new();
    for (pair, price_data) in all_price_data {
        if price_data.price > 0.0 {
            prices.insert(pair, price_data.price);
        }
    }

    HttpResponse::Ok().json(AllPricesResponse {
        status: "success".to_string(),
        timestamp: Utc::now().timestamp(),
        prices,
    })
}

#[post("/price/convert")]
pub async fn convert_amount(
    req: web::Json<ConvertRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    info!(
        "💱 Convert request: {} {} to {}",
        req.amount, req.from_symbol, req.to_symbol
    );

    let from_token = match TokenType::from_symbol(&req.from_symbol) {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid from_symbol: {}", e)
            }));
        }
    };

    let to_token = match TokenType::from_symbol(&req.to_symbol) {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid to_symbol: {}", e)
            }));
        }
    };

    match app_state
        .price_feed
        .convert_amount(&from_token, &to_token, req.amount)
        .await
    {
        Ok(output_amount) => {
            let rate = output_amount / req.amount;

            let response = ConvertResponse {
                from_symbol: from_token.symbol().to_string(),
                to_symbol: to_token.symbol().to_string(),
                input_amount: req.amount,
                output_amount,
                rate,
                timestamp: Utc::now().timestamp(),
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("Failed to convert amount: {}", e);
            HttpResponse::ServiceUnavailable().json(json!({
                "error": format!("Conversion failed: {}", e)
            }))
        }
    }
}

// ============================================================================
// METRICS & MONITORING
// ============================================================================

#[get("/metrics")]
pub async fn get_metrics(app_state: web::Data<AppState>) -> impl Responder {
    let metrics = app_state.bridge_coordinator.get_metrics().await;

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": {
            "ethereum_fills": metrics.ethereum_fills,
            "mantle_fills": metrics.mantle_fills,
            "successful_bridges": metrics.successful_bridges,
            "failed_intents": metrics.failed_intents,
            "volumes_by_token": metrics.volumes_by_token,
        }
    }))
}

#[get("/stats")]
pub async fn get_stats(app_state: web::Data<AppState>) -> impl Responder {
    match app_state.database.get_bridge_stats() {
        Ok(stats) => HttpResponse::Ok().json(StatsResponse {
            status: "success".to_string(),
            data: stats,
        }),
        Err(e) => {
            error!("Failed to get stats: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": "Failed to retrieve statistics"
            }))
        }
    }
}

#[get("/health")]
pub async fn health_check(app_state: web::Data<AppState>) -> impl Responder {
    // Check if critical components are healthy
    let ethereum_healthy = app_state.ethereum_relayer.health_check().await.is_ok();
    let mantle_healthy = app_state.mantle_relayer.health_check().await.is_ok();
    let db_healthy = app_state.database.health_check().is_ok();

    let overall_healthy = ethereum_healthy && mantle_healthy && db_healthy;

    let status_code = if overall_healthy {
        actix_web::http::StatusCode::OK
    } else {
        actix_web::http::StatusCode::SERVICE_UNAVAILABLE
    };

    HttpResponse::build(status_code).json(json!({
        "status": if overall_healthy { "healthy" } else { "unhealthy" },
        "timestamp": Utc::now().to_rfc3339(),
        "components": {
            "ethereum_relayer": if ethereum_healthy { "up" } else { "down" },
            "mantle_relayer": if mantle_healthy { "up" } else { "down" },
            "database": if db_healthy { "up" } else { "down" }
        }
    }))
}

#[get("/")]
pub async fn root() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "service": "Mantle-Ethereum Privacy Bridge",
        "version": "1.0.0",
        "status": "operational",
        "supported_chains": ["ethereum", "mantle"],
        "supported_tokens": ["ETH", "USDC", "USDT", "WETH", "MNT"]
    }))
}
