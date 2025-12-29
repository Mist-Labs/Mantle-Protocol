use actix_web::{HttpRequest, HttpResponse, web};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::{
    AppState,
    api::model::{IndexerEventRequest, IndexerEventResponse},
    models::model::IntentStatus,
};

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// HMAC VALIDATION
// ============================================================================

pub fn validate_hmac(
    req: &HttpRequest,
    body: &web::Bytes,
    app_state: &web::Data<AppState>,
) -> Result<(), HttpResponse> {
    let timestamp = match req.headers().get("x-timestamp") {
        Some(ts) => match ts.to_str() {
            Ok(s) => s,
            Err(_) => {
                return Err(HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": "Invalid timestamp header"
                })));
            }
        },
        None => {
            return Err(HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Missing x-timestamp header"
            })));
        }
    };

    let provided_signature = match req.headers().get("x-signature") {
        Some(sig) => match sig.to_str() {
            Ok(s) => s,
            Err(_) => {
                return Err(HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": "Invalid signature header"
                })));
            }
        },
        None => {
            return Err(HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Missing x-signature header"
            })));
        }
    };

    // Validate timestamp is recent (within 5 minutes)
    let current_timestamp = chrono::Utc::now().timestamp();
    let request_timestamp: i64 = match timestamp.parse() {
        Ok(ts) => ts,
        Err(_) => {
            return Err(HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Invalid timestamp format"
            })));
        }
    };

    let time_diff = (current_timestamp - request_timestamp).abs();
    if time_diff > 300 {
        // 5 minutes
        return Err(HttpResponse::Unauthorized().json(json!({
            "success": false,
            "message": "Request timestamp too old or in future"
        })));
    }

    let hmac_secret = &app_state.config.server.hmac_secret;

    let body_str = match std::str::from_utf8(body) {
        Ok(s) => s,
        Err(_) => {
            return Err(HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Invalid UTF-8 in body"
            })));
        }
    };

    let message = format!("{}{}", timestamp, body_str);

    let mut mac =
        HmacSha256::new_from_slice(hmac_secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if provided_signature != expected_signature {
        error!(
            "Invalid HMAC signature. Expected: {}, Got: {}",
            expected_signature, provided_signature
        );
        return Err(HttpResponse::Unauthorized().json(json!({
            "success": false,
            "message": "Invalid signature"
        })));
    }

    Ok(())
}

// ============================================================================
// EVENT HANDLERS
// ============================================================================

pub async fn handle_intent_created_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("📝 Processing intent_created event on {}", request.chain);

    // Extract intent_id from event_data
    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId in event_data".to_string(),
                error: None,
            });
        }
    };

    // Extract commitment from event_data
    let commitment = match request
        .event_data
        .get("commitment")
        .and_then(|v| v.as_str())
    {
        Some(c) => c,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing commitment in event_data".to_string(),
                error: None,
            });
        }
    };

    info!(
        "🔐 Intent created: {} | Commitment: {}",
        intent_id,
        &commitment[..commitment.len().min(16)]
    );

    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            // Update intent with on-chain confirmation
            intent.source_commitment = Some(commitment.to_string());
            intent.status = IntentStatus::Committed;
            intent.updated_at = chrono::Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent {}: {}", intent_id, e);
                return HttpResponse::InternalServerError().json(IndexerEventResponse {
                    success: false,
                    message: "Failed to update intent".to_string(),
                    error: Some(e.to_string()),
                });
            }

            // Add commitment to Merkle tree
            let chain_id = if request.chain == "ethereum" { 1 } else { 5000 };

            match app_state
                .merkle_manager
                .append_commitment(commitment, chain_id)
                .await
            {
                Ok(index) => {
                    info!(
                        "✅ Commitment added to {} Merkle tree at index {}",
                        request.chain, index
                    );

                    // Record event
                    if let Err(e) = app_state.database.record_intent_event(
                        intent_id,
                        "intent_created",
                        &request.chain,
                        &request.transaction_hash,
                        request.block_number,
                    ) {
                        error!("Failed to record event: {}", e);
                    }

                    HttpResponse::Ok().json(IndexerEventResponse {
                        success: true,
                        message: format!("Intent {} committed on {}", intent_id, request.chain),
                        error: None,
                    })
                }
                Err(e) => {
                    error!("Failed to add commitment to Merkle tree: {}", e);
                    HttpResponse::InternalServerError().json(IndexerEventResponse {
                        success: false,
                        message: "Failed to process commitment".to_string(),
                        error: Some(e.to_string()),
                    })
                }
            }
        }
        Ok(None) => {
            warn!("Intent {} not found in database", intent_id);
            HttpResponse::NotFound().json(IndexerEventResponse {
                success: false,
                message: "Intent not found".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Database error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

pub async fn handle_intent_filled_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("✅ Processing intent_filled event on {}", request.chain);

    // Extract intent_id from event_data
    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId in event_data".to_string(),
                error: None,
            });
        }
    };

    // Extract solver from event_data
    let solver = match request.event_data.get("solver").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing solver in event_data".to_string(),
                error: None,
            });
        }
    };

    info!(
        "🎯 Intent filled: {} | Solver: {} | Chain: {}",
        intent_id, solver, request.chain
    );

    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(_intent)) => {
            if let Err(e) = app_state.database.update_intent_with_solver(
                intent_id,
                solver,
                IntentStatus::Filled,
            ) {
                error!("Failed to update intent with solver {}: {}", intent_id, e);
                return HttpResponse::InternalServerError().json(IndexerEventResponse {
                    success: false,
                    message: "Failed to update intent".to_string(),
                    error: Some(e.to_string()),
                });
            }

            if let Err(e) = app_state
                .database
                .update_dest_fill_txid(intent_id, &request.transaction_hash)
            {
                error!("Failed to update dest_fill_txid: {}", e);
            }

            // Record fill event
            if let Err(e) = app_state.database.record_intent_event(
                intent_id,
                "intent_filled",
                &request.chain,
                &request.transaction_hash,
                request.block_number,
            ) {
                error!("Failed to record fill event: {}", e);
            }

            info!("✅ Intent {} marked as filled", intent_id);

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Intent {} filled on {}", intent_id, request.chain),
                error: None,
            })
        }
        Ok(None) => {
            warn!("Intent {} not found", intent_id);
            HttpResponse::NotFound().json(IndexerEventResponse {
                success: false,
                message: "Intent not found".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Database error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

pub async fn handle_intent_marked_filled_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!(
        "✅ Processing intent_marked_filled event on {}",
        request.chain
    );

    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId in event_data".to_string(),
                error: None,
            });
        }
    };

    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            // Update to reflect solver was paid
            intent.source_complete_txid = Some(request.transaction_hash.clone());
            intent.status = IntentStatus::SolverPaid; // New status
            intent.updated_at = chrono::Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent {}: {}", intent_id, e);
                return HttpResponse::InternalServerError().json(IndexerEventResponse {
                    success: false,
                    message: "Failed to update intent".to_string(),
                    error: Some(e.to_string()),
                });
            }

            // Record the mark filled event
            if let Err(e) = app_state.database.record_intent_event(
                intent_id,
                "intent_marked_filled",
                &request.chain,
                &request.transaction_hash,
                request.block_number,
            ) {
                error!("Failed to record mark filled event: {}", e);
            }

            info!("✅ Intent {} solver paid on source chain", intent_id);

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Solver paid for intent {} on {}", intent_id, request.chain),
                error: None,
            })
        }
        Ok(None) => {
            warn!("Intent {} not found", intent_id);
            HttpResponse::NotFound().json(IndexerEventResponse {
                success: false,
                message: "Intent not found".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Database error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

pub async fn handle_intent_refunded_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("♻️ Processing intent_refunded event on {}", request.chain);

    // Extract intent_id from event_data
    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId in event_data".to_string(),
                error: None,
            });
        }
    };

    info!("♻️ Intent refunded: {}", intent_id);

    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            // Update intent status
            intent.status = IntentStatus::Refunded;
            intent.updated_at = chrono::Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent {}: {}", intent_id, e);
                return HttpResponse::InternalServerError().json(IndexerEventResponse {
                    success: false,
                    message: "Failed to update intent".to_string(),
                    error: Some(e.to_string()),
                });
            }

            // Record refund event
            if let Err(e) = app_state.database.record_intent_event(
                intent_id,
                "intent_refunded",
                &request.chain,
                &request.transaction_hash,
                request.block_number,
            ) {
                error!("Failed to record refund event: {}", e);
            }

            info!("✅ Intent {} marked as refunded", intent_id);

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Intent {} refunded", intent_id),
                error: None,
            })
        }
        Ok(None) => {
            warn!("Intent {} not found", intent_id);
            HttpResponse::NotFound().json(IndexerEventResponse {
                success: false,
                message: "Intent not found".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Database error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

pub async fn handle_withdrawal_claimed_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!(
        "💸 Processing withdrawal_claimed event on {}",
        request.chain
    );

    // Extract intent_id from event_data
    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId in event_data".to_string(),
                error: None,
            });
        }
    };

    // Extract nullifier from event_data
    let nullifier = match request.event_data.get("nullifier").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing nullifier in event_data".to_string(),
                error: None,
            });
        }
    };

    info!(
        "💸 Withdrawal claimed: {} | Nullifier: {}",
        intent_id,
        &nullifier[..nullifier.len().min(16)]
    );

    // Record withdrawal event
    if let Err(e) = app_state.database.record_intent_event(
        intent_id,
        "withdrawal_claimed",
        &request.chain,
        &request.transaction_hash,
        request.block_number,
    ) {
        error!("Failed to record withdrawal event: {}", e);
    }

    // Store nullifier usage to prevent double-spending
    if let Err(e) =
        app_state
            .database
            .record_nullifier_usage(nullifier, intent_id, &request.transaction_hash)
    {
        error!("Failed to record nullifier usage: {}", e);
    }

    info!("✅ Withdrawal claimed for intent {}", intent_id);

    HttpResponse::Ok().json(IndexerEventResponse {
        success: true,
        message: format!("Withdrawal claimed for intent {}", intent_id),
        error: None,
    })
}

pub async fn handle_root_synced_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("🌳 Processing root_synced event on {}", request.chain);

    // Extract root from event_data
    let root = match request.event_data.get("root").and_then(|v| v.as_str()) {
        Some(r) => r,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing root in event_data".to_string(),
                error: None,
            });
        }
    };

    // Extract chainId from event_data
    let chain_id = match request.event_data.get("chainId").and_then(|v| v.as_u64()) {
        Some(id) => id as u32,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing chainId in event_data".to_string(),
                error: None,
            });
        }
    };

    info!(
        "🌳 Root synced: {} for chain {} on {}",
        &root[..root.len().min(16)],
        chain_id,
        request.chain
    );

    // Record root sync in database
    if let Err(e) = app_state.database.record_root_sync(
        &format!("{}_{}", request.chain, chain_id),
        root,
        &request.transaction_hash,
    ) {
        error!("Failed to record root sync: {}", e);
        return HttpResponse::InternalServerError().json(IndexerEventResponse {
            success: false,
            message: "Failed to record root sync".to_string(),
            error: Some(e.to_string()),
        });
    }

    info!("✅ Root sync recorded");

    HttpResponse::Ok().json(IndexerEventResponse {
        success: true,
        message: "Root sync recorded".to_string(),
        error: None,
    })
}

// New handler for intent_registered event (from PrivateSettlement)
pub async fn handle_intent_registered_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("📋 Processing intent_registered event on {}", request.chain);

    // Extract intent_id from event_data
    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId in event_data".to_string(),
                error: None,
            });
        }
    };

    info!("📋 Intent registered on destination: {}", intent_id);

    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            // Update intent with destination registration
            intent.dest_registration_txid = Some(request.transaction_hash.clone());
            intent.status = IntentStatus::Registered;
            intent.updated_at = chrono::Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent {}: {}", intent_id, e);
                return HttpResponse::InternalServerError().json(IndexerEventResponse {
                    success: false,
                    message: "Failed to update intent".to_string(),
                    error: Some(e.to_string()),
                });
            }

            // Record registration event
            if let Err(e) = app_state.database.record_intent_event(
                intent_id,
                "intent_registered",
                &request.chain,
                &request.transaction_hash,
                request.block_number,
            ) {
                error!("Failed to record registration event: {}", e);
            }

            info!("✅ Intent {} marked as registered", intent_id);

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Intent {} registered on {}", intent_id, request.chain),
                error: None,
            })
        }
        Ok(None) => {
            warn!("Intent {} not found", intent_id);
            HttpResponse::NotFound().json(IndexerEventResponse {
                success: false,
                message: "Intent not found".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Database error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}
