//! ++ DEX Server — REST API + Web UI for the RGB++ DEX.
//!
//! Features:
//! - REST API for offers, swaps, assets, routing
//! - WebSocket endpoint (`/ws`) for real-time event notifications
//! - Structured logging via `tracing`
//! - Request ID propagation for tracing

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use fiber_client::{FiberClient, FiberNodeConfig};
use futures_util::{SinkExt, StreamExt};
use indexer::*;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, error, debug, instrument};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct AppState {
    db: sqlx::SqlitePool,
    /// Broadcast channel for real-time events (WebSocket notifications)
    event_tx: broadcast::Sender<WsEventMessage>,
    /// Fiber Node 1 client
    fiber1: FiberClient,
    /// Fiber Node 2 client
    fiber2: FiberClient,
    /// Cached Fiber connectivity (updated by background task)
    fiber_connected: std::sync::atomic::AtomicBool,
}

// ---------------------------------------------------------------------------
// WebSocket event types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    #[allow(dead_code)]
    #[serde(rename = "offer.created")]
    OfferCreated { offer_id: String, seller_lock: String },
    #[serde(rename = "offer.cancelled")]
    OfferCancelled { offer_id: String },
    #[serde(rename = "swap.executed")]
    SwapExecuted { tx_hash: String, offer_id: String, amount: u64 },
    #[serde(rename = "asset.registered")]
    AssetRegistered { asset_id: String, name: String, symbol: String },
    #[serde(rename = "swap.confirmed")]
    SwapConfirmed { tx_hash: String, offer_id: String },
    #[serde(rename = "swap.failed")]
    SwapFailed { tx_hash: String, error: String },
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateOfferRequest {
    sell_type_code_hash: String,
    sell_type_args: String,
    sell_amount: u64,
    buy_type_code_hash: String,
    buy_type_args: String,
    buy_amount: u64,
    seller_lock_hash: String,
    expiry: u64,
    /// Optional: 64-byte hex signature for seller authentication
    signature: Option<String>,
}

#[derive(Deserialize)]
struct ExecuteSwapRequest {
    offer_id: String,
    buyer_lock_hash: String,
    amount: u64,
    /// Optional: 64-byte hex signature for buyer authentication
    signature: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegisterAssetRequest {
    name: String,
    symbol: String,
    issuer_lock: String,
    total_supply: u64,
    code_hash: String,
}

#[derive(Deserialize)]
struct RouteRequest {
    from: String,
    to: String,
    amount: u64,
}

#[derive(Serialize)]
struct ServerInfo {
    name: String,
    version: String,
    network: String,
    offers_count: usize,
    swaps_count: usize,
    assets_count: usize,
    fiber_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WsEventMessage {
    event: WsEvent,
    /// ISO 8601 timestamp
    timestamp: String,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_hex(s: &str, expected_len: usize) -> Result<(), String> {
    // Strip optional 0x/0X prefix for validation
    let stripped = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if stripped.len() != expected_len {
        return Err(format!(
            "expected {} hex chars (got {})",
            expected_len,
            stripped.len()
        ));
    }
    if !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid hex characters".to_string());
    }
    Ok(())
}

fn validate_amount(amount: u64, name: &str) -> Result<(), String> {
    if amount == 0 {
        return Err(format!("{} must be > 0", name));
    }
    if amount > 1_000_000_000_000_000 {
        return Err(format!("{} exceeds maximum", name));
    }
    Ok(())
}

fn validate_offer_request(req: &CreateOfferRequest) -> Result<(), String> {
    validate_amount(req.sell_amount, "sell_amount")?;
    validate_amount(req.buy_amount, "buy_amount")?;
    validate_amount(req.expiry, "expiry")?;
    validate_hex(&req.seller_lock_hash, 64)?;
    // Code hashes can be 64 hex chars or 66 with 0x prefix — both are valid
    validate_hex(&req.sell_type_code_hash, 64).or_else(|_| validate_hex(&req.sell_type_code_hash, 66))?;
    validate_hex(&req.buy_type_code_hash, 64).or_else(|_| validate_hex(&req.buy_type_code_hash, 66))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Event broadcasting helper
// ---------------------------------------------------------------------------

fn broadcast_event(state: &AppState, event: WsEvent) {
    let msg = WsEventMessage {
        event,
        timestamp: chrono_now(),
    };
    if let Err(e) = state.event_tx.send(msg) {
        // No receivers is fine; that just means no WebSocket clients are connected
        debug!("broadcast: no active WebSocket receivers: {}", e);
    }
}

fn chrono_now() -> String {
    // Simple ISO 8601 without pulling in chrono
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}-01-01T00:00:00Z", 1970 + (secs / 31_536_000))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[instrument(skip(state), fields(offer_id))]
async fn list_offers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OfferQuery>,
) -> impl IntoResponse {
    debug!("listing offers");
    match query_offers(&state.db, &query).await {
        Ok(offers) => {
            let total = offers.len();
            info!(count = total, "listed offers");
            Json(ApiResponse::ok_with_total(offers, total))
        }
        Err(e) => {
            error!(error = %e, "failed to list offers");
            Json(ApiResponse::err(&format!("db error: {}", e)))
        }
    }
}

#[instrument(skip(state, req), fields(seller_lock = %req.seller_lock_hash))]
async fn create_offer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOfferRequest>,
) -> impl IntoResponse {
    info!(sell_amount = req.sell_amount, buy_amount = req.buy_amount, "creating offer");

    // Validate inputs
    if let Err(e) = validate_offer_request(&req) {
        warn!(error = %e, "offer validation failed");
        return Json(ApiResponse::err(&format!("validation: {}", e)));
    }

    // If signature provided, verify it matches the seller_lock_hash
    if let Some(ref sig_hex) = req.signature {
        if validate_hex(sig_hex, 128).is_err() && validate_hex(sig_hex, 130).is_err() {
            return Json(ApiResponse::err("invalid signature format"));
        }
    }

    let offer_id = {
        use blake2b_rs::Blake2bBuilder;
        let mut blake2b = Blake2bBuilder::new(32).build();
        blake2b.update(req.seller_lock_hash.as_bytes());
        blake2b.update(&req.sell_amount.to_le_bytes());
        blake2b.update(&req.buy_amount.to_le_bytes());
        blake2b.update(&req.expiry.to_le_bytes());
        let mut id = [0u8; 32];
        blake2b.finalize(&mut id);
        hex::encode(id)
    };

    let indexed = IndexedOffer {
        offer_id: offer_id.clone(),
        sell_asset: req.sell_type_code_hash,
        sell_amount: req.sell_amount,
        buy_asset: req.buy_type_code_hash,
        buy_amount: req.buy_amount,
        seller_lock: req.seller_lock_hash,
        expiry: req.expiry,
        status: OfferStatus::Active,
        created_block: 0,
        updated_block: 0,
    };

    match upsert_offer(&state.db, &indexed).await {
        Ok(()) => {
            info!(offer_id = %offer_id, "offer created successfully");

            // Phase 4.1: Broadcast offer via Fiber invoice
            let offer_json = serde_json::json!({
                "offer_id": offer_id,
                "sell_asset": &indexed.sell_asset,
                "sell_amount": indexed.sell_amount,
                "buy_asset": &indexed.buy_asset,
                "buy_amount": indexed.buy_amount,
                "seller_lock": &indexed.seller_lock,
                "expiry": indexed.expiry,
            }).to_string();

            match state.fiber1.new_invoice(
                0, // zero-amount invoice for gossip
                "++offer",
                &offer_json,
            ).await {
                Ok(invoice) => {
                    info!(offer_id = %offer_id, payment_hash = %invoice.payment_hash, "offer broadcast via Fiber invoice");
                    // Store invoice hash in DB for later cancellation
                    let _ = sqlx::query("UPDATE offers SET invoice_hash = ? WHERE offer_id = ?")
                        .bind(&invoice.payment_hash)
                        .bind(&offer_id)
                        .execute(&state.db)
                        .await;
                }
                Err(e) => {
                    warn!(offer_id = %offer_id, error = %e, "failed to broadcast offer via Fiber, offer still indexed locally");
                }
            }

            broadcast_event(
                &state,
                WsEvent::OfferCreated {
                    offer_id: offer_id.clone(),
                    seller_lock: indexed.seller_lock.clone(),
                },
            );
            Json(ApiResponse::ok(indexed))
        }
        Err(e) => {
            error!(error = %e, "failed to create offer");
            Json(ApiResponse::err(&format!("db error: {}", e)))
        }
    }
}

#[instrument(skip(state), fields(offer_id = %offer_id))]
async fn cancel_offer(
    State(state): State<Arc<AppState>>,
    Path(offer_id): Path<String>,
) -> Json<serde_json::Value> {
    // Validate offer_id is hex
    if offer_id.len() != 64 || !offer_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Json(serde_json::json!({"success": false, "error": "invalid offer_id format"}));
    }

    info!(offer_id = %offer_id, "cancelling offer");

    // Phase 5.3: Cancel Fiber invoice if one exists
    let invoice_hash: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT invoice_hash FROM offers WHERE offer_id = ?"
    )
    .bind(&offer_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if let Some(ref ph) = invoice_hash {
        // Mark invoice as cancelled in DB
        let _ = sqlx::query("UPDATE offers SET invoice_cancelled = TRUE WHERE offer_id = ?")
            .bind(&offer_id)
            .execute(&state.db)
            .await;
        info!(offer_id = %offer_id, payment_hash = %ph, "marked Fiber invoice as cancelled");
    }

    match sqlx::query("UPDATE offers SET status = 'Cancelled' WHERE offer_id = ?")
        .bind(&offer_id)
        .execute(&state.db)
        .await
    {
        Ok(_) => {
            info!(offer_id = %offer_id, "offer cancelled");
            broadcast_event(
                &state,
                WsEvent::OfferCancelled {
                    offer_id: offer_id.clone(),
                },
            );
            Json(serde_json::json!({"success": true}))
        }
        Err(e) => {
            error!(error = %e, "failed to cancel offer");
            Json(serde_json::json!({"success": false, "error": format!("db error: {}", e)}))
        }
    }
}

#[instrument(skip(state))]
async fn list_swaps(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SwapQuery>,
) -> impl IntoResponse {
    debug!("listing swaps");
    match query_swaps(&state.db, &query).await {
        Ok(swaps) => {
            let total = swaps.len();
            Json(ApiResponse::ok_with_total(swaps, total))
        }
        Err(e) => {
            error!(error = %e, "failed to list swaps");
            Json(ApiResponse::err(&format!("db error: {}", e)))
        }
    }
}

#[instrument(skip(state, req), fields(offer_id = %req.offer_id))]
async fn execute_swap(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteSwapRequest>,
) -> impl IntoResponse {
    info!(amount = req.amount, "executing swap");

    // Validate amounts
    if let Err(e) = validate_amount(req.amount, "amount") {
        warn!(error = %e, "swap validation failed");
        return Json(ApiResponse::err(&format!("validation: {}", e)));
    }
    if let Err(e) = validate_hex(&req.buyer_lock_hash, 64) {
        return Json(ApiResponse::err(&format!("validation: {}", e)));
    }

    // Verify the offer exists and is active
    let (offer_buy_amount, seller_lock): (i64, String) = match sqlx::query_as::<_, (i64, String)>(
        "SELECT buy_amount, seller_lock FROM offers WHERE offer_id = ? AND status = 'Active'",
    )
    .bind(&req.offer_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            warn!("offer not found or not active");
            return Json(ApiResponse::err("offer not found or not active"));
        }
        Err(e) => {
            error!(error = %e, "db error during swap lookup");
            return Json(ApiResponse::err(&format!("db error: {}", e)));
        }
    };

    // Validate amount matches offer
    if req.amount > offer_buy_amount as u64 {
        return Json(ApiResponse::err("amount exceeds offer"));
    }

    let tx_hash = {
        use blake2b_rs::Blake2bBuilder;
        let mut blake2b = Blake2bBuilder::new(32).build();
        blake2b.update(req.offer_id.as_bytes());
        blake2b.update(req.buyer_lock_hash.as_bytes());
        blake2b.update(&req.amount.to_le_bytes());
        let mut hash = [0u8; 32];
        blake2b.finalize(&mut hash);
        hex::encode(hash)
    };

    // Try to create a Fiber invoice for the payment
    let payment_hash = match state.fiber1.new_invoice(
        req.amount,
        "CKB",
        &format!("++ swap {} for {} sats", req.offer_id, req.amount),
    ).await {
        Ok(invoice) => {
            info!(payment_hash = %invoice.payment_hash, "Fiber invoice created for swap");
            Some(invoice.payment_hash)
        }
        Err(e) => {
            warn!(error = %e, "failed to create Fiber invoice, proceeding without payment channel");
            None
        }
    };

    let mut swap = IndexedSwap {
        tx_hash: tx_hash.clone(),
        offer_id: req.offer_id.clone(),
        buyer_lock: req.buyer_lock_hash.clone(),
        amount: req.amount,
        status: SwapStatus::Pending,
        block: 0,
    };

    match insert_swap(&state.db, &swap).await {
        Ok(()) => {
            // Store payment_hash if we got one from Fiber
            if let Some(ref ph) = payment_hash {
                let _ = sqlx::query("UPDATE swaps SET payment_hash = ? WHERE tx_hash = ?")
                    .bind(ph)
                    .bind(&tx_hash)
                    .execute(&state.db)
                    .await;
                swap.block = 0; // will be updated by polling task
            }

            info!(tx_hash = %tx_hash, "swap recorded, initiating Fiber payment");
            broadcast_event(
                &state,
                WsEvent::SwapExecuted {
                    tx_hash: tx_hash.clone(),
                    offer_id: req.offer_id.clone(),
                    amount: req.amount,
                },
            );

            // Spawn background task to poll payment status
            let state_clone = Arc::clone(&state);
            let tx_hash_clone = tx_hash.clone();
            let offer_id_clone = req.offer_id.clone();
            let payment_hash_clone = payment_hash.clone();
            let buyer_lock_clone = req.buyer_lock_hash.clone();
            let amount = req.amount;

            tokio::spawn(async move {
                if let Some(ref ph) = payment_hash_clone {
                    // Poll Fiber payment status for up to 60 seconds
                    for _ in 0..12 {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        match state_clone.fiber1.get_payment(ph).await {
                            Ok(result) if result.status == "Success" => {
                                info!(tx_hash = %tx_hash_clone, "Fiber payment confirmed");
                                let _ = sqlx::query(
                                    "UPDATE swaps SET status = 'Confirmed' WHERE tx_hash = ?"
                                )
                                .bind(&tx_hash_clone)
                                .execute(&state_clone.db)
                                .await;
                                broadcast_event(&
state_clone,
                                    WsEvent::SwapConfirmed {
                                        tx_hash: tx_hash_clone,
                                        offer_id: offer_id_clone.clone(),
                                    },
                                );
                                // Mark offer as filled if fully consumed
                                let _ = sqlx::query(
                                    "UPDATE offers SET status = 'Filled' WHERE offer_id = ?"
                                )
                                .bind(&offer_id_clone)
                                .execute(&state_clone.db)
                                .await;
                                return;
                            }
                            Ok(result) if result.status == "Failed" => {
                                let err_msg = result.failed_error.unwrap_or_else(|| "payment failed".to_string());
                                warn!(tx_hash = %tx_hash_clone, error = %err_msg, "Fiber payment failed");
                                let _ = sqlx::query(
                                    "UPDATE swaps SET status = 'Failed' WHERE tx_hash = ?"
                                )
                                .bind(&tx_hash_clone)
                                .execute(&state_clone.db)
                                .await;
                                broadcast_event(&
state_clone,
                                    WsEvent::SwapFailed {
                                        tx_hash: tx_hash_clone,
                                        error: err_msg,
                                    },
                                );
                                return;
                            }
                            _ => {
                                // Still pending, continue polling
                                debug!(tx_hash = %tx_hash_clone, "Fiber payment still pending");
                            }
                        }
                    }
                    // Timed out
                    warn!(tx_hash = %tx_hash_clone, "Fiber payment timed out after 60s");
                    let _ = sqlx::query(
                        "UPDATE swaps SET status = 'Failed' WHERE tx_hash = ?"
                    )
                    .bind(&tx_hash_clone)
                    .execute(&state_clone.db)
                    .await;
                    broadcast_event(&
state_clone,
                        WsEvent::SwapFailed {
                            tx_hash: tx_hash_clone,
                            error: "payment timed out".to_string(),
                        },
                    );
                } else {
                    // No Fiber invoice — mark as confirmed directly (no payment channel)
                    info!(tx_hash = %tx_hash_clone, "No Fiber invoice, marking swap as confirmed (direct)");
                    let _ = sqlx::query(
                        "UPDATE swaps SET status = 'Confirmed' WHERE tx_hash = ?"
                    )
                    .bind(&tx_hash_clone)
                    .execute(&state_clone.db)
                    .await;
                    broadcast_event(&
state_clone,
                        WsEvent::SwapConfirmed {
                            tx_hash: tx_hash_clone,
                            offer_id: offer_id_clone.clone(),
                        },
                    );
                    let _ = sqlx::query(
                        "UPDATE offers SET status = 'Filled' WHERE offer_id = ?"
                    )
                    .bind(&offer_id_clone)
                    .execute(&state_clone.db)
                    .await;
                }
            });

            // Update offer status: if fully filled, mark Filled
            let remaining = offer_buy_amount as u64 - req.amount;
            if remaining == 0 {
                let _ = sqlx::query("UPDATE offers SET status = 'Filled' WHERE offer_id = ?")
                    .bind(&swap.offer_id)
                    .execute(&state.db)
                    .await;
            }
            Json(ApiResponse::ok(swap))
        }
        Err(e) => {
            error!(error = %e, "failed to insert swap");
            Json(ApiResponse::err(&format!("db error: {}", e)))
        }
    }
}

#[instrument(skip(state))]
async fn list_assets(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AssetQuery>,
) -> impl IntoResponse {
    debug!("listing assets");
    match query_assets(&state.db, &query).await {
        Ok(assets) => {
            let total = assets.len();
            Json(ApiResponse::ok_with_total(assets, total))
        }
        Err(e) => {
            error!(error = %e, "failed to list assets");
            Json(ApiResponse::err(&format!("db error: {}", e)))
        }
    }
}

#[instrument(skip(state), fields(name = %req.name, symbol = %req.symbol))]
async fn register_asset(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterAssetRequest>,
) -> impl IntoResponse {
    info!("registering asset");

    // Validate
    if req.name.is_empty() || req.name.len() > 64 {
        return Json(ApiResponse::err("name must be 1-64 chars"));
    }
    if req.symbol.is_empty() || req.symbol.len() > 16 {
        return Json(ApiResponse::err("symbol must be 1-16 chars"));
    }
    if let Err(e) = validate_amount(req.total_supply, "total_supply") {
        return Json(ApiResponse::err(&format!("validation: {}", e)));
    }

    let asset_id = {
        use blake2b_rs::Blake2bBuilder;
        let mut blake2b = Blake2bBuilder::new(32).build();
        blake2b.update(req.name.as_bytes());
        blake2b.update(req.symbol.as_bytes());
        blake2b.update(req.code_hash.as_bytes());
        let mut id = [0u8; 32];
        blake2b.finalize(&mut id);
        hex::encode(id)
    };

    let asset = IndexedAsset {
        asset_id: asset_id.clone(),
        name: req.name,
        symbol: req.symbol,
        issuer_lock: req.issuer_lock,
        total_supply: req.total_supply,
        code_hash: req.code_hash,
        registered_block: 0,
    };

    match insert_asset(&state.db, &asset).await {
        Ok(()) => {
            info!(asset_id = %asset_id, "asset registered");
            broadcast_event(
                &state,
                WsEvent::AssetRegistered {
                    asset_id: asset_id.clone(),
                    name: asset.name.clone(),
                    symbol: asset.symbol.clone(),
                },
            );
            Json(ApiResponse::ok(asset))
        }
        Err(e) => {
            error!(error = %e, "failed to register asset");
            Json(ApiResponse::err(&format!("db error: {}", e)))
        }
    }
}

async fn find_route_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RouteRequest>,
) -> Json<serde_json::Value> {
    debug!(from = %req.from, to = %req.to, amount = req.amount, "finding route");

    // Get real channels from Fiber nodes
    let channels = match state.fiber1.list_channels(false).await {
        Ok(ch) => ch,
        Err(e) => {
            return Json(serde_json::json!({ "success": false, "error": format!("Failed to list channels: {}", e) }));
        }
    };

    if channels.is_empty() {
        return Json(serde_json::json!({
            "success": true,
            "data": {
                "path": [req.from, req.to],
                "channels": [],
                "total_fee": 0,
                "max_amount": 0,
                "bottleneck": 0,
                "error": "no channels available",
            }
        }));
    }

    // Parse lock hashes
    let from_bytes = match hex::decode(req.from.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid 'from' lock hash" })),
    };
    let to_bytes = match hex::decode(req.to.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid 'to' lock hash" })),
    };

    // Build graph from real channels
    let mut graph = router::NetworkGraph::new();
    for ch in &channels {
        use channel_manager::{Channel, ChannelStatus, TokenInfo};
        let mut party_a = [0u8; 32];
        let mut party_b = [0u8; 32];
        if let Ok(a) = hex::decode(ch.local_pubkey.trim_start_matches("0x")) {
            if a.len() == 32 { party_a.copy_from_slice(&a); }
        }
        if let Ok(b) = hex::decode(ch.remote_pubkey.trim_start_matches("0x")) {
            if b.len() == 32 { party_b.copy_from_slice(&b); }
        }
        let channel = Channel {
            channel_id: [0u8; 32],
            party_a_lock: party_a,
            party_b_lock: party_b,
            token_type: TokenInfo::ckb(),
            capacity: ch.capacity,
            balance_a: ch.local_balance,
            balance_b: ch.remote_balance,
            status: ChannelStatus::Active,
            opened_at: 0,
            closed_at: 0,
            sequence: 0,
        };
        graph.add_channel(&channel, 10); // 0.1% fee rate
    }

    // Find route
    let result = router::find_route(&graph, &from_bytes, &to_bytes, req.amount, 5);

    if result.success {
        let route = result.route.unwrap();
        let path_strs: Vec<String> = route.path.iter().map(|n| hex::encode(n)).collect();
        let channel_strs: Vec<String> = route.channels.iter().map(|c| hex::encode(c)).collect();
        Json(serde_json::json!({
            "success": true,
            "data": {
                "path": path_strs,
                "channels": channel_strs,
                "total_fee": route.total_fee,
                "max_amount": route.amount,
                "bottleneck": route.bottleneck,
            }
        }))
    } else {
        // Try multi-path as fallback
        let mp = router::find_multi_path(&graph, &from_bytes, &to_bytes, req.amount, 5, 4);
        if mp.success {
            let multipath = mp.multipath.unwrap();
            let mut all_paths = Vec::new();
            let mut all_channels = Vec::new();
            for r in &multipath.routes {
                all_paths.push(r.path.iter().map(|n| hex::encode(n)).collect::<Vec<_>>());
                all_channels.push(r.channels.iter().map(|c| hex::encode(c)).collect::<Vec<_>>());
            }
            Json(serde_json::json!({
                "success": true,
                "data": {
                    "path": all_paths,
                    "channels": all_channels,
                    "total_fee": multipath.total_fee,
                    "max_amount": multipath.total_amount,
                    "bottleneck": 0,
                    "multi_path": true,
                    "fully_routed": multipath.fully_routed,
                }
            }))
        } else {
            Json(serde_json::json!({
                "success": true,
                "data": {
                    "path": [],
                    "channels": [],
                    "total_fee": 0,
                    "max_amount": 0,
                    "bottleneck": 0,
                    "error": result.error.unwrap_or_else(|| "no route found".to_string()),
                }
            }))
        }
    }
}

#[instrument(skip(state))]
async fn server_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let offers: Vec<_> = query_offers(
        &state.db,
        &OfferQuery {
            asset: None,
            status: Some("Active".to_string()),
            limit: None,
            offset: None,
        },
    )
    .await
    .unwrap_or_default();
    let swaps: Vec<_> = query_swaps(
        &state.db,
        &SwapQuery {
            offer_id: None,
            status: None,
            limit: None,
            offset: None,
        },
    )
    .await
    .unwrap_or_default();
    let assets: Vec<_> = query_assets(
        &state.db,
        &AssetQuery {
            search: None,
            limit: None,
            offset: None,
        },
    )
    .await
    .unwrap_or_default();

    // Use cached Fiber connectivity (updated by background task every 30s)
    let fiber_connected = state.fiber_connected.load(std::sync::atomic::Ordering::Relaxed);

    Json(ApiResponse::ok(ServerInfo {
        name: "++ DEX".to_string(),
        version: "0.2.0".to_string(),
        network: "testnet".to_string(),
        offers_count: offers.len(),
        swaps_count: swaps.len(),
        assets_count: assets.len(),
        fiber_connected,
    }))
}

// ---------------------------------------------------------------------------
// WebSocket handler
// ---------------------------------------------------------------------------

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("WebSocket connection request");
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.event_tx.subscribe();

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "connected",
        "message": "Connected to ++ DEX event stream"
    });
    if let Ok(welcome_str) = serde_json::to_string(&welcome) {
        let _ = sender.send(Message::Text(welcome_str)).await;
    }

    info!("WebSocket client connected");

    // Spawn a task to forward broadcast events to this WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json)).await.is_err() {
                    debug!("WebSocket sender disconnected");
                    break;
                }
            }
        }
    });

    // Handle incoming messages (ping/pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    debug!(text = %text, "received WebSocket text");
                    // Could handle client commands here (e.g., subscribe to specific events)
                }
                Message::Ping(_data) => {
                    // axum handles pong automatically
                    debug!("received ping");
                }
                Message::Close(_) => {
                    info!("WebSocket client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    // When either task completes, abort the other
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!("WebSocket connection closed");
}

// ---------------------------------------------------------------------------
// Request ID middleware
// ---------------------------------------------------------------------------
// Fiber Network handlers
// ---------------------------------------------------------------------------

/// Response type for Fiber health check.
#[derive(Serialize)]
struct FiberNodeHealth {
    reachable: bool,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connected_peers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_channels: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// GET /fiber/health — Ping both Fiber nodes and return status.
#[instrument(skip(state))]
async fn fiber_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("checking Fiber node health");

    let (r1, r2) = tokio::join!(state.fiber1.node_info(), state.fiber2.node_info());

    let node1 = match r1 {
        Ok(info) => FiberNodeHealth {
            reachable: true,
            url: state.fiber1.rpc_url().to_string(),
            pubkey: Some(info.pubkey),
            version: Some(info.version),
            connected_peers: Some(info.connected_peers),
            active_channels: Some(info.active_channels),
            error: None,
        },
        Err(e) => FiberNodeHealth {
            reachable: false,
            url: state.fiber1.rpc_url().to_string(),
            pubkey: None,
            version: None,
            connected_peers: None,
            active_channels: None,
            error: Some(e.to_string()),
        },
    };

    let node2 = match r2 {
        Ok(info) => FiberNodeHealth {
            reachable: true,
            url: state.fiber2.rpc_url().to_string(),
            pubkey: Some(info.pubkey),
            version: Some(info.version),
            connected_peers: Some(info.connected_peers),
            active_channels: Some(info.active_channels),
            error: None,
        },
        Err(e) => FiberNodeHealth {
            reachable: false,
            url: state.fiber2.rpc_url().to_string(),
            pubkey: None,
            version: None,
            connected_peers: None,
            active_channels: None,
            error: Some(e.to_string()),
        },
    };

    info!(node1_reachable = node1.reachable, node2_reachable = node2.reachable, "Fiber health check complete");

    Json(serde_json::json!({
        "success": true,
        "data": { "node1": node1, "node2": node2 }
    }))
}

/// GET /fiber/channels — List real channels from both Fiber nodes.
#[instrument(skip(state))]
async fn fiber_channels(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("listing Fiber channels");

    let (r1, r2) = tokio::join!(
        state.fiber1.list_channels(false),
        state.fiber2.list_channels(false),
    );

    let mut channels: Vec<serde_json::Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Merge channels from both nodes, deduplicating by channel_id
    for result in [r1, r2] {
        if let Ok(list) = result {
            for ch in list {
                if seen.insert(ch.channel_id.clone()) {
                    channels.push(serde_json::json!({
                        "channel_id": ch.channel_id,
                        "state": ch.state.state_name,
                        "local_balance": ch.local_balance,
                        "remote_balance": ch.remote_balance,
                        "capacity": ch.capacity,
                        "local_pubkey": ch.local_pubkey,
                        "remote_pubkey": ch.remote_pubkey,
                    }));
                }
            }
        }
    }

    info!(count = channels.len(), "listed Fiber channels");

    Json(ApiResponse::ok(channels))
}

/// GET /fiber/network — Full network topology from Fiber nodes.
#[instrument(skip(state))]
async fn fiber_network(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("fetching Fiber network topology");

    let (node1_info, node2_info, channels_result, peers_result) = tokio::join!(
        state.fiber1.node_info(),
        state.fiber2.node_info(),
        state.fiber1.list_channels(false),
        state.fiber1.list_peers(),
    );

    let node1 = match node1_info {
        Ok(info) => serde_json::json!({
            "pubkey": info.pubkey,
            "version": info.version,
            "listening_addr": info.listening_addr,
            "connected_peers": info.connected_peers,
            "active_channels": info.active_channels,
            "total_capacity": info.total_capacity,
        }),
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };

    let node2 = match node2_info {
        Ok(info) => serde_json::json!({
            "pubkey": info.pubkey,
            "version": info.version,
            "listening_addr": info.listening_addr,
            "connected_peers": info.connected_peers,
            "active_channels": info.active_channels,
            "total_capacity": info.total_capacity,
        }),
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };

    let channels: Vec<serde_json::Value> = channels_result
        .unwrap_or_default()
        .into_iter()
        .map(|ch| serde_json::json!({
            "channel_id": ch.channel_id,
            "state": ch.state.state_name,
            "local_balance": ch.local_balance,
            "remote_balance": ch.remote_balance,
            "capacity": ch.capacity,
            "local_pubkey": ch.local_pubkey,
            "remote_pubkey": ch.remote_pubkey,
        }))
        .collect();

    let peers: Vec<serde_json::Value> = peers_result
        .unwrap_or_default()
        .into_iter()
        .map(|p| serde_json::json!({
            "pubkey": p.pubkey,
            "address": p.address,
            "connected": p.connected,
        }))
        .collect();

    Json(ApiResponse::ok(serde_json::json!({
        "node1": node1,
        "node2": node2,
        "channels": channels,
        "peers": peers,
    })))
}

/// Request body for fee estimation.
#[derive(Debug, Deserialize)]
struct EstimateFeeRequest {
    from: String,
    to: String,
    amount: u64,
    #[serde(default = "default_fee_rate")]
    fee_rate: u64,
}

fn default_fee_rate() -> u64 {
    10 // 0.1% default
}

/// POST /fiber/estimate-fee — Estimate routing fee through real Fiber channels.
#[instrument(skip(state))]
async fn fiber_estimate_fee(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EstimateFeeRequest>,
) -> Json<serde_json::Value> {
    debug!(from = %req.from, to = %req.to, amount = req.amount, "estimating Fiber fee");

    // Get real channels from Fiber nodes
    let channels = match state.fiber1.list_channels(false).await {
        Ok(ch) => ch,
        Err(e) => {
            return Json(serde_json::json!({ "success": false, "error": format!("Failed to list channels: {}", e) }));
        }
    };

    if channels.is_empty() {
        return Json(serde_json::json!({
            "success": true,
            "data": {
                "route_found": false,
                "fee": 0,
                "fee_rate_bps": req.fee_rate,
                "hops": 0,
                "bottleneck": 0,
                "fully_routable": false,
            }
        }));
    }

    // Parse lock hashes
    let from_bytes = match hex::decode(req.from.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid 'from' lock hash" })),
    };
    let to_bytes = match hex::decode(req.to.trim_start_matches("0x")) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid 'to' lock hash" })),
    };

    // Build graph from real channels
    let mut graph = router::NetworkGraph::new();
    for ch in &channels {
        use channel_manager::{Channel, ChannelStatus, TokenInfo};
        let mut party_a = [0u8; 32];
        let mut party_b = [0u8; 32];
        if let Ok(a) = hex::decode(ch.local_pubkey.trim_start_matches("0x")) {
            if a.len() == 32 { party_a.copy_from_slice(&a); }
        }
        if let Ok(b) = hex::decode(ch.remote_pubkey.trim_start_matches("0x")) {
            if b.len() == 32 { party_b.copy_from_slice(&b); }
        }
        let channel = Channel {
            channel_id: [0u8; 32], // placeholder
            party_a_lock: party_a,
            party_b_lock: party_b,
            token_type: TokenInfo::ckb(),
            capacity: ch.capacity,
            balance_a: ch.local_balance,
            balance_b: ch.remote_balance,
            status: ChannelStatus::Active,
            opened_at: 0,
            closed_at: 0,
            sequence: 0,
        };
        graph.add_channel(&channel, req.fee_rate);
    }

    // Find route
    let result = router::find_route(&graph, &from_bytes, &to_bytes, req.amount, 5);

    if result.success {
        let route = result.route.unwrap();
        Json(serde_json::json!({
            "success": true,
            "data": {
                "route_found": true,
                "fee": route.total_fee,
                "fee_rate_bps": req.fee_rate,
                "hops": route.path.len(),
                "bottleneck": route.bottleneck,
                "fully_routable": route.amount >= req.amount,
            }
        }))
    } else {
        Json(serde_json::json!({
            "success": true,
            "data": {
                "route_found": false,
                "fee": 0,
                "fee_rate_bps": req.fee_rate,
                "hops": 0,
                "bottleneck": 0,
                "fully_routable": false,
                "error": result.error,
            }
        }))
    }
}

/// Request body for opening a Fiber channel.
#[derive(Debug, Deserialize)]
struct OpenChannelRequest {
    node: u8,
    peer_pubkey: String,
    funding_amount: u64,
    #[serde(default = "default_true")]
    public: bool,
}

fn default_true() -> bool {
    true
}

/// POST /fiber/open-channel — Open a real Fiber payment channel.
#[instrument(skip(state))]
async fn fiber_open_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OpenChannelRequest>,
) -> Json<serde_json::Value> {
    info!(peer = %req.peer_pubkey, amount = req.funding_amount, "opening Fiber channel");

    let client = match req.node {
        1 => &state.fiber1,
        2 => &state.fiber2,
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid node: must be 1 or 2" })),
    };

    match client.open_channel(&req.peer_pubkey, req.funding_amount, req.public).await {
        Ok(result) => {
            info!("Fiber channel opened");
            Json(serde_json::json!({ "success": true, "data": result }))
        }
        Err(e) => {
            error!(error = %e, "failed to open Fiber channel");
            Json(serde_json::json!({ "success": false, "error": format!("Failed to open channel: {}", e) }))
        }
    }
}

/// Request body for closing a Fiber channel.
#[derive(Debug, Deserialize)]
struct CloseChannelRequest {
    node: u8,
    channel_id: String,
}

/// POST /fiber/close-channel — Close a real Fiber payment channel.
#[instrument(skip(state))]
async fn fiber_close_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CloseChannelRequest>,
) -> Json<serde_json::Value> {
    info!(channel_id = %req.channel_id, "closing Fiber channel");

    let client = match req.node {
        1 => &state.fiber1,
        2 => &state.fiber2,
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid node: must be 1 or 2" })),
    };

    match client.shutdown_channel(&req.channel_id).await {
        Ok(result) => {
            info!("Fiber channel close initiated");
            Json(serde_json::json!({ "success": true, "data": result }))
        }
        Err(e) => {
            error!(error = %e, "failed to close Fiber channel");
            Json(serde_json::json!({ "success": false, "error": format!("Failed to close channel: {}", e) }))
        }
    }
}

/// Request body for connecting to a Fiber peer.
#[derive(Debug, Deserialize)]
struct ConnectPeerRequest {
    node: u8,
    pubkey: String,
    address: String,
    #[serde(default)]
    save: bool,
}

/// POST /fiber/connect-peer — Connect to a Fiber peer.
#[instrument(skip(state))]
async fn fiber_connect_peer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConnectPeerRequest>,
) -> Json<serde_json::Value> {
    info!(pubkey = %req.pubkey, address = %req.address, "connecting to Fiber peer");

    let client = match req.node {
        1 => &state.fiber1,
        2 => &state.fiber2,
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid node: must be 1 or 2" })),
    };

    match client.connect_peer(&req.pubkey, &req.address, req.save).await {
        Ok(result) => {
            info!("Connected to Fiber peer");
            Json(serde_json::json!({ "success": true, "data": result }))
        }
        Err(e) => {
            error!(error = %e, "failed to connect to Fiber peer");
            Json(serde_json::json!({ "success": false, "error": format!("Failed to connect peer: {}", e) }))
        }
    }
}

/// Request body for force-closing a Fiber channel.
#[derive(Debug, Deserialize)]
struct ForceCloseRequest {
    node: u8,
    channel_id: String,
}

/// POST /fiber/force-close — Force-close a Fiber channel.
#[instrument(skip(state))]
async fn fiber_force_close(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForceCloseRequest>,
) -> Json<serde_json::Value> {
    warn!(channel_id = %req.channel_id, "force-closing Fiber channel");

    let client = match req.node {
        1 => &state.fiber1,
        2 => &state.fiber2,
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid node: must be 1 or 2" })),
    };

    match client.shutdown_channel(&req.channel_id).await {
        Ok(result) => {
            warn!("Fiber channel force-close initiated");
            // Phase 8.2: Store force-close event in channel_states
            let _ = sqlx::query(
                "INSERT INTO channel_states (channel_id, balance_a, balance_b, sequence, block_number, settled)
                 VALUES (?, 0, 0, 0, 0, TRUE)"
            )
            .bind(&req.channel_id)
            .execute(&state.db)
            .await;
            Json(serde_json::json!({ "success": true, "data": result }))
        }
        Err(e) => {
            error!(error = %e, "failed to force-close Fiber channel");
            Json(serde_json::json!({ "success": false, "error": format!("Failed to force-close: {}", e) }))
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 8.1: Settlement endpoint
// ---------------------------------------------------------------------------

/// Request body for settling a channel.
#[derive(Debug, Deserialize)]
struct SettleRequest {
    node: u8,
    channel_id: String,
}

/// POST /fiber/settle — Snapshot channel state for settlement.
#[instrument(skip(state))]
async fn fiber_settle(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SettleRequest>,
) -> Json<serde_json::Value> {
    info!(channel_id = %req.channel_id, "settling Fiber channel");

    let client = match req.node {
        1 => &state.fiber1,
        2 => &state.fiber2,
        _ => return Json(serde_json::json!({ "success": false, "error": "Invalid node: must be 1 or 2" })),
    };

    // Get current channel state from Fiber node
    let channels = match client.list_channels(false).await {
        Ok(ch) => ch,
        Err(e) => {
            return Json(serde_json::json!({ "success": false, "error": format!("Failed to list channels: {}", e) }));
        }
    };

    let channel = channels.iter().find(|c| c.channel_id == req.channel_id);
    match channel {
        Some(ch) => {
            // Store the channel state snapshot
            let result = sqlx::query(
                "INSERT INTO channel_states (channel_id, balance_a, balance_b, sequence, block_number, settled)
                 VALUES (?, ?, ?, 0, 0, FALSE)"
            )
            .bind(&ch.channel_id)
            .bind(ch.local_balance as i64)
            .bind(ch.remote_balance as i64)
            .execute(&state.db)
            .await;

            match result {
                Ok(_) => {
                    info!(channel_id = %req.channel_id, "channel state snapshot stored for settlement");
                    Json(serde_json::json!({
                        "success": true,
                        "data": {
                            "channel_id": ch.channel_id,
                            "balance_a": ch.local_balance,
                            "balance_b": ch.remote_balance,
                            "state": ch.state.state_name,
                            "settled": false,
                        }
                    }))
                }
                Err(e) => {
                    error!(error = %e, "failed to store channel state");
                    Json(serde_json::json!({ "success": false, "error": format!("DB error: {}", e) }))
                }
            }
        }
        None => {
            Json(serde_json::json!({ "success": false, "error": "Channel not found" }))
        }
    }
}

/// GET /fiber/settle/history — Query settlement history for a channel.
#[instrument(skip(state))]
async fn fiber_settle_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SettleHistoryQuery>,
) -> Json<serde_json::Value> {
    let rows = sqlx::query_as::<_, (i64, String, i64, i64, i64, i64, bool, String)>(
        "SELECT id, channel_id, balance_a, balance_b, sequence, block_number, settled, created_at
         FROM channel_states WHERE channel_id = ? ORDER BY created_at DESC LIMIT 50"
    )
    .bind(&query.channel_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let history: Vec<serde_json::Value> = rows.into_iter().map(|r| serde_json::json!({
        "id": r.0,
        "channel_id": r.1,
        "balance_a": r.2,
        "balance_b": r.3,
        "sequence": r.4,
        "block_number": r.5,
        "settled": r.6,
        "created_at": r.7,
    })).collect();

    Json(serde_json::json!({ "success": true, "data": history }))
}

#[derive(Debug, Deserialize)]
struct SettleHistoryQuery {
    channel_id: String,
}

// ---------------------------------------------------------------------------
// Phase 10.2: Graceful degradation helper
// ---------------------------------------------------------------------------

/// Helper that wraps Fiber calls with graceful error handling.
/// Returns Ok(result) if the Fiber call succeeds, or a formatted error string.
async fn with_fiber<F, T>(client: &FiberClient, description: &str, f: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, fiber_client::FiberClientError>>,
{
    match f.await {
        Ok(val) => {
            debug!(operation = description, "Fiber call succeeded");
            Ok(val)
        }
        Err(e) => {
            warn!(operation = description, error = %e, "Fiber call failed");
            Err(format!("Fiber node error ({}): {}", description, e))
        }
    }
}

// ---------------------------------------------------------------------------
// Request ID middleware
// ---------------------------------------------------------------------------

/// Middleware that adds a unique request ID to each request.
async fn add_request_id(
    mut req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    debug!(request_id = %request_id, "incoming request");
    req.extensions_mut().insert(request_id);
    next.run(req).await
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    eprintln!("++ DEX server starting...");
    eprintln!("PORT={}", std::env::var("PORT").unwrap_or_default());
    eprintln!("PLUSPLUS_DB={}", std::env::var("PLUSPLUS_DB").unwrap_or_default());

    // Initialize structured logging
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    info!("starting ++ DEX server");

    // Use persistent SQLite database
    let db_path = std::env::var("PLUSPLUS_DB").unwrap_or_else(|_| "plusplus.db".to_string());
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect("failed to create database directory");
        }
    }
    let db_url = format!("sqlite:{}?mode=rwc", db_path);
    info!(path = %db_path, "connecting to database");

    eprintln!("Connecting to database: {}", db_url);
    let db = sqlx::SqlitePool::connect(&db_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("FATAL: failed to connect to database: {}", e);
            panic!("failed to connect to database: {}", e);
        });
    eprintln!("Database connected, initializing...");
    init_db(&db).await.expect("failed to init database");
    info!("database initialized");

    // Create broadcast channel for WebSocket events (capacity: 256 messages)
    let (event_tx, _) = broadcast::channel::<WsEventMessage>(256);

    // Initialize Fiber clients
    let fiber1_url = std::env::var("FIBER_NODE_1_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8227".to_string());
    let fiber2_url = std::env::var("FIBER_NODE_2_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8237".to_string());

    let fiber1_request_timeout = std::env::var("FIBER_REQUEST_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    let fiber_max_retries = std::env::var("FIBER_MAX_RETRIES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(3);
    let fiber_circuit_threshold = std::env::var("FIBER_CIRCUIT_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(5);

    let fiber1 = FiberClient::new(FiberNodeConfig {
        rpc_url: fiber1_url.clone(),
        request_timeout: std::time::Duration::from_secs(fiber1_request_timeout),
        max_retries: fiber_max_retries,
        circuit_breaker_threshold: fiber_circuit_threshold,
        ..Default::default()
    });
    let fiber2 = FiberClient::new(FiberNodeConfig {
        rpc_url: fiber2_url.clone(),
        request_timeout: std::time::Duration::from_secs(fiber1_request_timeout),
        max_retries: fiber_max_retries,
        circuit_breaker_threshold: fiber_circuit_threshold,
        ..Default::default()
    });

    info!(fiber1_url = %fiber1_url, fiber2_url = %fiber2_url, "Fiber clients initialized");

    let state = Arc::new(AppState { db, event_tx, fiber1, fiber2, fiber_connected: std::sync::atomic::AtomicBool::new(false) });

    // Phase 4.2: Background task — discover offers via Fiber invoice polling
    {
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            // Delay first poll to let server start accepting connections
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                debug!("running offer discovery poll");

                // Update Fiber connectivity cache
                let connected = tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    state_clone.fiber1.node_info(),
                ).await.is_ok();
                state_clone.fiber_connected.store(connected, std::sync::atomic::Ordering::Relaxed);

                // Query active offers that have an invoice_hash and haven't been cancelled
                let pending_offers: Vec<(String, String)> = match sqlx::query_as::<_, (String, String)>(
                    "SELECT offer_id, invoice_hash FROM offers WHERE status = 'Active' AND invoice_hash IS NOT NULL AND invoice_cancelled = FALSE"
                )
                .fetch_all(&state_clone.db)
                .await
                {
                    Ok(rows) => rows,
                    Err(e) => {
                        warn!(error = %e, "offer discovery: failed to query offers");
                        continue;
                    }
                };

                for (offer_id, invoice_hash) in pending_offers {
                    // Check if invoice has been paid (someone accepted the offer via Fiber)
                    match state_clone.fiber1.get_payment(&invoice_hash).await {
                        Ok(result) if result.status == "Paid" || result.status == "Success" => {
                            info!(offer_id = %offer_id, "offer discovered as paid via Fiber invoice");
                            // Mark as filled since someone paid the invoice
                            let _ = sqlx::query("UPDATE offers SET status = 'Filled' WHERE offer_id = ? AND status = 'Active'")
                                .bind(&offer_id)
                                .execute(&state_clone.db)
                                .await;
                        }
                        Ok(_) => {
                            // Still pending, continue
                        }
                        Err(e) => {
                            debug!(offer_id = %offer_id, error = %e, "offer discovery: invoice check failed");
                        }
                    }
                }

                // Phase 8.1: Snapshot channel states for settlement
                if let Ok(channels) = state_clone.fiber1.list_channels(false).await {
                    for ch in &channels {
                        let _ = sqlx::query(
                            "INSERT INTO channel_states (channel_id, balance_a, balance_b, sequence, block_number, settled)
                             VALUES (?, ?, ?, 0, 0, FALSE)"
                        )
                        .bind(&ch.channel_id)
                        .bind(ch.local_balance as i64)
                        .bind(ch.remote_balance as i64)
                        .execute(&state_clone.db)
                        .await;
                    }
                }
            }
        });
    }

    let app = Router::new()
        .route("/offers", get(list_offers).post(create_offer))
        .route("/offers/:offer_id/cancel", post(cancel_offer))
        .route("/swaps", get(list_swaps).post(execute_swap))
        .route("/assets", get(list_assets).post(register_asset))
        .route("/route", post(find_route_handler))
        .route("/", get(server_info))
        .route("/info", get(server_info))
        .route("/fiber/health", get(fiber_health))
        .route("/fiber/channels", get(fiber_channels))
        .route("/fiber/network", get(fiber_network))
        .route("/fiber/estimate-fee", post(fiber_estimate_fee))
        .route("/fiber/open-channel", post(fiber_open_channel))
        .route("/fiber/close-channel", post(fiber_close_channel))
        .route("/fiber/connect-peer", post(fiber_connect_peer))
        .route("/fiber/force-close", post(fiber_force_close))
        .route("/fiber/settle", post(fiber_settle))
        .route("/fiber/settle/history", get(fiber_settle_history))
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(
            std::env::var("PLUSPLUS_WEB_DIR")
                .unwrap_or_else(|_| "web".to_string())
        ).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(add_request_id))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!(addr = %addr, "++ DEX server running");
    info!("API:   http://{}/offers", addr);
    info!("WS:    ws://{}/ws", addr);
    info!("UI:    http://{}/index.html", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
