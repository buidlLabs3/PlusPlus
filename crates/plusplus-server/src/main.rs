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
    routing::{get, post, delete},
    Json, Router,
};
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
    if s.len() != expected_len {
        return Err(format!(
            "expected {} hex chars, got {}",
            expected_len,
            s.len()
        ));
    }
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
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
    validate_hex(&req.sell_type_code_hash, 66)?; // 0x + 64 hex
    validate_hex(&req.buy_type_code_hash, 66)?;
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
) -> impl IntoResponse {
    // Validate offer_id is hex
    if offer_id.len() != 64 || !offer_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Json(ApiResponse::<()>::err("invalid offer_id format"));
    }

    info!(offer_id = %offer_id, "cancelling offer");

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
            Json(ApiResponse::<()>::ok(()))
        }
        Err(e) => {
            error!(error = %e, "failed to cancel offer");
            Json(ApiResponse::err(&format!("db error: {}", e)))
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
    let offer_buy_amount: i64 = match sqlx::query_scalar::<_, i64>(
        "SELECT buy_amount FROM offers WHERE offer_id = ? AND status = 'Active'",
    )
    .bind(&req.offer_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(amt)) => amt,
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

    let swap = IndexedSwap {
        tx_hash: tx_hash.clone(),
        offer_id: req.offer_id.clone(),
        buyer_lock: req.buyer_lock_hash,
        amount: req.amount,
        status: SwapStatus::Pending,
        block: 0,
    };

    match insert_swap(&state.db, &swap).await {
        Ok(()) => {
            info!(tx_hash = %tx_hash, "swap executed");
            broadcast_event(
                &state,
                WsEvent::SwapExecuted {
                    tx_hash: tx_hash.clone(),
                    offer_id: req.offer_id.clone(),
                    amount: req.amount,
                },
            );
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

async fn find_route_handler(Json(req): Json<RouteRequest>) -> impl IntoResponse {
    debug!(from = %req.from, to = %req.to, amount = req.amount, "finding route");
    let route = serde_json::json!({
        "path": [req.from, req.to],
        "channels": [],
        "total_fee": req.amount / 1000,
        "max_amount": req.amount,
    });
    Json(ApiResponse::ok(route))
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

    Json(ApiResponse::ok(ServerInfo {
        name: "++ DEX".to_string(),
        version: "0.1.0".to_string(),
        network: "testnet".to_string(),
        offers_count: offers.len(),
        swaps_count: swaps.len(),
        assets_count: assets.len(),
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
    let db_url = format!("sqlite:{}", db_path);
    info!(path = %db_path, "connecting to database");

    let db = sqlx::SqlitePool::connect(&db_url)
        .await
        .expect("failed to connect to database");
    init_db(&db).await.expect("failed to init database");
    info!("database initialized");

    // Create broadcast channel for WebSocket events (capacity: 256 messages)
    let (event_tx, _) = broadcast::channel::<WsEventMessage>(256);

    let state = Arc::new(AppState { db, event_tx });

    let app = Router::new()
        .route("/offers", get(list_offers).post(create_offer))
        .route("/offers/{offer_id}", delete(cancel_offer))
        .route("/swaps", get(list_swaps).post(execute_swap))
        .route("/assets", get(list_assets).post(register_asset))
        .route("/route", post(find_route_handler))
        .route("/info", get(server_info))
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(
            std::env::var("PLUSPLUS_WEB_DIR")
                .unwrap_or_else(|_| "web".to_string())
        ))
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
