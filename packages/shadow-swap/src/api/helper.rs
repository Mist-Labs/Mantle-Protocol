use actix_web::{HttpRequest, HttpResponse, web};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::{AppState, api::model::{IndexerEventRequest, IndexerEventResponse}, models::model::IntentStatus};


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
    info!(
        "📝 Processing intent_created event on {}",
        request.chain
    );

    let intent_id = match &request.intent_id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intent_id".to_string(),
                error: None,
            });
        }
    };

    let commitment = match &request.commitment {
        Some(c) => c,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing commitment".to_string(),
                error: None,
            });
        }
    };

    info!(
        "🔐 Intent created: {} | Commitment: {}",
        intent_id,
        &commitment[..16]
    );


    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            // Update intent with on-chain confirmation
            intent.source_commitment = Some(commitment.clone());
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
                        request.block_number.unwrap_or(0),
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

    let intent_id = match &request.intent_id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intent_id".to_string(),
                error: None,
            });
        }
    };

    let solver = match &request.solver {
        Some(s) => s,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing solver address".to_string(),
                error: None,
            });
        }
    };

    info!(
        "🎯 Intent filled: {} | Solver: {} | Chain: {}",
        intent_id, solver, request.chain
    );

    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            // Update intent status
            intent.dest_fill_txid = Some(request.transaction_hash.clone());
            intent.status = IntentStatus::Filled;
            intent.updated_at = chrono::Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent {}: {}", intent_id, e);
                return HttpResponse::InternalServerError().json(IndexerEventResponse {
                    success: false,
                    message: "Failed to update intent".to_string(),
                    error: Some(e.to_string()),
                });
            }

            // Record fill event
            if let Err(e) = app_state.database.record_intent_event(
                intent_id,
                "intent_filled",
                &request.chain,
                &request.transaction_hash,
                request.block_number.unwrap_or(0),
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

pub async fn handle_intent_completed_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!(
        "🎉 Processing intent_completed event on {}",
        request.chain
    );

    let intent_id = match &request.intent_id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intent_id".to_string(),
                error: None,
            });
        }
    };

    info!("🏁 Intent completed: {}", intent_id);

    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            // Update intent status
            intent.source_complete_txid = Some(request.transaction_hash.clone());
            intent.status = IntentStatus::Completed;
            intent.updated_at = chrono::Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent {}: {}", intent_id, e);
                return HttpResponse::InternalServerError().json(IndexerEventResponse {
                    success: false,
                    message: "Failed to update intent".to_string(),
                    error: Some(e.to_string()),
                });
            }

            // Record completion event
            if let Err(e) = app_state.database.record_intent_event(
                intent_id,
                "intent_completed",
                &request.chain,
                &request.transaction_hash,
                request.block_number.unwrap_or(0),
            ) {
                error!("Failed to record completion event: {}", e);
            }

            // Update metrics
            let mut metrics = app_state.bridge_coordinator.metrics.write().await;
            metrics.successful_bridges += 1;

            info!("✅ Intent {} marked as completed", intent_id);

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Intent {} completed successfully", intent_id),
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

    let intent_id = match &request.intent_id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intent_id".to_string(),
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
                request.block_number.unwrap_or(0),
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

    let intent_id = match &request.intent_id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intent_id".to_string(),
                error: None,
            });
        }
    };

    let nullifier = match &request.nullifier {
        Some(n) => n,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing nullifier".to_string(),
                error: None,
            });
        }
    };

    info!(
        "💸 Withdrawal claimed: {} | Nullifier: {}",
        intent_id,
        &nullifier[..16]
    );

    // Record withdrawal event
    if let Err(e) = app_state.database.record_intent_event(
        intent_id,
        "withdrawal_claimed",
        &request.chain,
        &request.transaction_hash,
        request.block_number.unwrap_or(0),
    ) {
        error!("Failed to record withdrawal event: {}", e);
    }

    // Store nullifier usage to prevent double-spending
    if let Err(e) = app_state
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

    let root = match &request.root {
        Some(r) => r,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing root".to_string(),
                error: None,
            });
        }
    };

    let chain_id = match &request.dest_chain_id {
        Some(id) => *id,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing dest_chain_id".to_string(),
                error: None,
            });
        }
    };

    info!(
        "🌳 Root synced: {} for chain {} on {}",
        &root[..16],
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