use actix_web::{HttpRequest, HttpResponse, web};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::{
    AppState,
    api::model::{IndexerEventRequest, IndexerEventResponse},
    models::model::{Intent, IntentStatus},
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

// fn extract_chain_id(event_data: &serde_json::Map<String, serde_json::Value>) -> Option<u32> {
//     event_data.get("chainId").and_then(|v| {
//         if let Some(num) = v.as_u64() {
//             return Some(num as u32);
//         }

//         if let Some(s) = v.as_str() {
//             return s.parse::<u32>().ok();
//         }
//         None
//     })
// }

fn get_chain_id(chain: &str) -> u32 {
    match chain {
        "ethereum" => 11155111,
        "mantle" => 5003,
        _ => 0,
    }
}

fn store_raw_event(
    app_state: &web::Data<AppState>,
    event_type: &str,
    request: &IndexerEventRequest,
    intent_id: Option<&str>,
) -> Result<(), String> {
    let chain_id = get_chain_id(&request.chain);
    let event_id = format!(
        "{}_{}_{}_{}",
        event_type, request.chain, request.transaction_hash, request.log_index,
    );

    app_state
        .database
        .store_bridge_event(
            &event_id,
            intent_id,
            event_type,
            serde_json::to_value(&request.event_data).unwrap_or_default(),
            chain_id as i32,
            request.block_number as i64,
            Some(request.log_index as i32),
            &request.transaction_hash,
        )
        .map_err(|e| {
            // Check if it's a duplicate key error (idempotency)
            if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
                info!("Event {} already exists (idempotent)", event_id);
                return "duplicate".to_string();
            }
            e.to_string()
        })
}

// ============================================================================
// EVENT HANDLERS
// ============================================================================

pub async fn handle_intent_created_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("📝 Processing intent_created on {}", request.chain);

    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) if !id.is_empty() => id,
        _ => {
            error!("Missing or empty intentId");
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId in event_data".to_string(),
                error: None,
            });
        }
    };

    let commitment = match request
        .event_data
        .get("commitment")
        .and_then(|v| v.as_str())
    {
        Some(c) if !c.is_empty() => c,
        _ => {
            error!("Missing or empty commitment");
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing commitment in event_data".to_string(),
                error: None,
            });
        }
    };

    let source_token = request
        .event_data
        .get("sourceToken")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let dest_token = request
        .event_data
        .get("destToken")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let source_amount = request
        .event_data
        .get("sourceAmount")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let dest_amount = request
        .event_data
        .get("destAmount")
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let dest_chain = request
        .event_data
        .get("destChain")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let refund_address = request
        .event_data
        .get("refundAddress")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let block_number = Some(request.block_number as i64);
    let log_index = Some(request.log_index as i32);

    info!(
        "🔍 DEBUG: block_number = {:?}, log_index = {:?}",
        block_number, log_index
    );
    info!(
        "🔍 DEBUG: request.block_number = {}, request.log_index = {}",
        request.block_number, request.log_index
    );

    let intent = Intent {
        id: intent_id.to_string(),
        user_address: refund_address.to_string(),
        source_chain: request.chain.clone(),
        dest_chain: dest_chain.to_string(),
        source_token: source_token.to_string(),
        dest_token: dest_token.to_string(),
        amount: source_amount.to_string(),
        dest_amount: dest_amount.to_string(),
        source_commitment: Some(commitment.to_string()),
        dest_fill_txid: None,
        dest_registration_txid: None,
        source_complete_txid: None,
        status: IntentStatus::Committed,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deadline: (Utc::now().timestamp() + 3600) as u64,
        refund_address: Some(refund_address.to_string()),
        solver_address: None,
        block_number,
        log_index,
    };

    if let Err(e) = app_state.database.upsert_intent(&intent) {
        error!("Failed to upsert intent: {}", e);
        return HttpResponse::InternalServerError().json(IndexerEventResponse {
            success: false,
            message: "Failed to upsert intent".to_string(),
            error: Some(e.to_string()),
        });
    }

    info!("✅ Intent {} upserted", intent_id);

    match store_raw_event(app_state, "intent_created", request, Some(intent_id)) {
        Ok(()) => info!("✅ Raw event stored"),
        Err(e) if e == "duplicate" => info!("Event already stored (idempotent)"),
        Err(e) => {
            error!("Failed to store raw event: {}", e);
            return HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Failed to persist event".to_string(),
                error: Some(e),
            });
        }
    }

    // ✅ Event handler job done - return immediately
    HttpResponse::Ok().json(IndexerEventResponse {
        success: true,
        message: format!("Intent {} committed on {}", intent_id, request.chain),
        error: None,
    })
}

pub async fn handle_intent_filled_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("✅ Processing intent_filled on {}", request.chain);

    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) if !id.is_empty() => id,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId".to_string(),
                error: None,
            });
        }
    };

    let solver = match request.event_data.get("solver").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing solver".to_string(),
                error: None,
            });
        }
    };

    // STEP 1: Store raw event FIRST (idempotency)
    match store_raw_event(app_state, "intent_filled", request, Some(intent_id)) {
        Ok(()) => info!("✅ Raw fill event stored"),
        Err(e) if e == "duplicate" => {
            info!("Fill event already stored, but continuing to rebuild tree");
            // DON'T RETURN - continue to rebuild tree
        }
        Err(e) => {
            error!("CRITICAL: Failed to store raw fill event: {}", e);
            return HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Failed to persist fill event".to_string(),
                error: Some(e),
            });
        }
    }

    // STEP 2: Update intent status
    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            intent.status = IntentStatus::Filled;
            intent.solver_address = Some(solver.to_string());
            intent.dest_fill_txid = Some(request.transaction_hash.clone());
            intent.updated_at = Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent: {}", e);
            } else {
                info!("✅ Intent {} marked as filled", intent_id);
            }
        }
        Ok(None) => {
            warn!("Intent {} not found in intents table", intent_id);
        }
        Err(e) => {
            error!("Database error: {}", e);
        }
    }

    info!("🔄 Force rebuilding fill tree for chain {}", request.chain);

    let rebuild_result = match request.chain.as_str() {
        "ethereum" | "11155111" => app_state.merkle_manager.rebuild_ethereum_fills_tree().await,
        "mantle" | "5003" => app_state.merkle_manager.rebuild_mantle_fills_tree().await,
        _ => {
            error!("Unknown chain for fill event: {}", request.chain);
            return HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Fill recorded but unknown chain".to_string(),
                error: None,
            });
        }
    };

    match rebuild_result {
        Ok(()) => info!("✅ Fill tree rebuilt successfully"),
        Err(e) => error!("❌ Failed to rebuild fill tree: {}", e),
    }

    HttpResponse::Ok().json(IndexerEventResponse {
        success: true,
        message: format!("Intent {} filled by {}", intent_id, solver),
        error: None,
    })
}

pub async fn handle_intent_settled_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("✅ Processing intent_settled on {}", request.chain);

    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) if !id.is_empty() => id,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId".to_string(),
                error: None,
            });
        }
    };

    // STEP 1: Store raw event FIRST
    match store_raw_event(app_state, "intent_settled", request, Some(intent_id)) {
        Ok(()) => info!("✅ Raw settled event stored"),
        Err(e) if e == "duplicate" => {
            return HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Settled event already processed".to_string(),
                error: None,
            });
        }
        Err(e) => {
            error!("CRITICAL: Failed to store raw settled event: {}", e);
            return HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Failed to persist settled event".to_string(),
                error: Some(e),
            });
        }
    }

    // STEP 2: Update intent status
    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            intent.status = IntentStatus::SolverPaid;
            intent.source_complete_txid = Some(request.transaction_hash.clone());
            intent.updated_at = Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent: {}", e);
            } else {
                info!("✅ Intent {} marked as settled", intent_id);
            }

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Intent {} settled", intent_id),
                error: None,
            })
        }
        Ok(None) => {
            warn!("Intent {} not found", intent_id);
            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Settled event recorded (intent not found)".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Settled event recorded (DB error)".to_string(),
                error: None,
            })
        }
    }
}

pub async fn handle_intent_refunded_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("♻️ Processing intent_refunded on {}", request.chain);

    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) if !id.is_empty() => id,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId".to_string(),
                error: None,
            });
        }
    };

    // STEP 1: Store raw event FIRST
    match store_raw_event(app_state, "intent_refunded", request, Some(intent_id)) {
        Ok(()) => info!("✅ Raw refunded event stored"),
        Err(e) if e == "duplicate" => {
            return HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Refunded event already processed".to_string(),
                error: None,
            });
        }
        Err(e) => {
            error!("CRITICAL: Failed to store raw refunded event: {}", e);
            return HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Failed to persist refunded event".to_string(),
                error: Some(e),
            });
        }
    }

    // STEP 2: Update intent status
    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            intent.status = IntentStatus::Refunded;
            intent.updated_at = Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent: {}", e);
            } else {
                info!("✅ Intent {} marked as refunded", intent_id);
            }

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Intent {} refunded", intent_id),
                error: None,
            })
        }
        Ok(None) => {
            warn!("Intent {} not found", intent_id);
            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Refunded event recorded (intent not found)".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Refunded event recorded (DB error)".to_string(),
                error: None,
            })
        }
    }
}

pub async fn handle_withdrawal_claimed_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("💸 Processing withdrawal_claimed on {}", request.chain);

    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) if !id.is_empty() => id,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId".to_string(),
                error: None,
            });
        }
    };

    let nullifier = match request.event_data.get("nullifier").and_then(|v| v.as_str()) {
        Some(n) if !n.is_empty() => n,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing nullifier".to_string(),
                error: None,
            });
        }
    };

    // STEP 1: Store raw event FIRST
    match store_raw_event(app_state, "withdrawal_claimed", request, Some(intent_id)) {
        Ok(()) => info!("✅ Raw withdrawal event stored"),
        Err(e) if e == "duplicate" => {
            return HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Withdrawal event already processed".to_string(),
                error: None,
            });
        }
        Err(e) => {
            error!("CRITICAL: Failed to store raw withdrawal event: {}", e);
            return HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Failed to persist withdrawal event".to_string(),
                error: Some(e),
            });
        }
    }

    // STEP 2: Record nullifier usage
    if let Err(e) =
        app_state
            .database
            .record_nullifier_usage(nullifier, intent_id, &request.transaction_hash)
    {
        error!("Failed to record nullifier: {}", e);
    }

    // STEP 3: Update intent status to UserClaimed
    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            intent.status = IntentStatus::UserClaimed;
            intent.updated_at = Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent: {}", e);
            } else {
                info!("✅ Intent {} marked as user claimed", intent_id);
            }
        }
        _ => {
            warn!("Intent {} not found for withdrawal", intent_id);
        }
    }

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
    info!("🌳 Processing root_synced on {}", request.chain);

    let event_map = match request.event_data.as_object() {
        Some(map) => map,
        None => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "event_data is not a valid object".to_string(),
                error: None,
            });
        }
    };

    let event_kind = event_map
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("COMMITMENT");
    info!("🔍 Root Sync Type identified as: {}", event_kind);

    let root = match event_map.get("root").and_then(|v| v.as_str()) {
        Some(r) if !r.is_empty() => r,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing root".to_string(),
                error: None,
            });
        }
    };

    let chain_id = match event_map
        .get("chainId")
        .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
    {
        Some(id) => id as u32,
        None => {
            error!("Missing chainId in root_synced event");
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing chainId".to_string(),
                error: None,
            });
        }
    };

    let sync_type = format!("{}_{}", request.chain, chain_id);

    // STEP 1: Store raw event FIRST
    match store_raw_event(app_state, "root_sync", request, None) {
        Ok(()) => info!("✅ Raw root sync event stored"),
        Err(e) if e == "duplicate" => {
            return HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Root sync already processed".to_string(),
                error: None,
            });
        }
        Err(e) => {
            error!("CRITICAL: Failed to store raw root sync event: {}", e);
            return HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Failed to persist root sync event".to_string(),
                error: Some(e),
            });
        }
    }

    // STEP 2: Insert into root_syncs table
    if let Err(e) = app_state
        .database
        .insert_root_sync(&sync_type, root, &request.transaction_hash)
    {
        error!("Failed to insert root sync: {}", e);
    } else {
        info!("✅ Root sync recorded in root_syncs table");
    }

    HttpResponse::Ok().json(IndexerEventResponse {
        success: true,
        message: "Root sync recorded".to_string(),
        error: None,
    })
}

pub async fn handle_intent_registered_event(
    app_state: &web::Data<AppState>,
    request: &IndexerEventRequest,
) -> HttpResponse {
    info!("📋 Processing intent_registered on {}", request.chain);

    let intent_id = match request.event_data.get("intentId").and_then(|v| v.as_str()) {
        Some(id) if !id.is_empty() => id,
        _ => {
            return HttpResponse::BadRequest().json(IndexerEventResponse {
                success: false,
                message: "Missing intentId".to_string(),
                error: None,
            });
        }
    };

    // STEP 1: Store raw event FIRST
    match store_raw_event(app_state, "intent_registered", request, Some(intent_id)) {
        Ok(()) => info!("✅ Raw registered event stored"),
        Err(e) if e == "duplicate" => {
            return HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Registered event already processed".to_string(),
                error: None,
            });
        }
        Err(e) => {
            error!("CRITICAL: Failed to store raw registered event: {}", e);
            return HttpResponse::InternalServerError().json(IndexerEventResponse {
                success: false,
                message: "Failed to persist registered event".to_string(),
                error: Some(e),
            });
        }
    }

    // STEP 2: Update intent status
    match app_state.database.get_intent_by_id(intent_id) {
        Ok(Some(mut intent)) => {
            intent.status = IntentStatus::Registered;
            intent.dest_registration_txid = Some(request.transaction_hash.clone());
            intent.updated_at = Utc::now();

            if let Err(e) = app_state.database.update_intent(&intent) {
                error!("Failed to update intent: {}", e);
            } else {
                info!("✅ Intent {} marked as registered", intent_id);
            }

            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: format!("Intent {} registered", intent_id),
                error: None,
            })
        }
        Ok(None) => {
            warn!("Intent {} not found", intent_id);
            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Registered event recorded (intent not found)".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::Ok().json(IndexerEventResponse {
                success: true,
                message: "Registered event recorded (DB error)".to_string(),
                error: None,
            })
        }
    }
}
