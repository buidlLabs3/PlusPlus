//! Fiber Client — Communicates with Fiber nodes via JSON-RPC.
//!
//! The Fiber Network Node (FNN) exposes a JSON-RPC interface on port 8227.
//! This client wraps that interface with:
//! - Configurable HTTP request timeouts
//! - Retry with exponential backoff
//! - Circuit breaker to avoid hammering a down node
//! - Structured logging via `tracing`

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Fiber node configuration.
#[derive(Debug, Clone)]
pub struct FiberNodeConfig {
    /// JSON-RPC endpoint (e.g., "http://localhost:8227")
    pub rpc_url: String,
    /// HTTP request timeout (default: 30s)
    pub request_timeout: Duration,
    /// Maximum number of retries (default: 3)
    pub max_retries: u32,
    /// Base delay for exponential backoff (default: 500ms)
    pub retry_base_delay: Duration,
    /// Circuit breaker: failures before opening (default: 5)
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker: time before half-open (default: 30s)
    pub circuit_breaker_reset: Duration,
}

impl Default for FiberNodeConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8227".to_string(),
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_base_delay: Duration::from_millis(500),
            circuit_breaker_threshold: 5,
            circuit_breaker_reset: Duration::from_secs(30),
        }
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

/// JSON-RPC request wrapper.
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// JSON-RPC response wrapper.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error.
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[allow(dead_code)]
    data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Fiber RPC types
// ---------------------------------------------------------------------------

/// Node information from the Fiber node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    #[serde(rename = "pubkey")]
    pub pubkey: String,
    #[serde(rename = "version", default)]
    pub version: String,
    #[serde(rename = "listening_addr", default)]
    pub listening_addr: String,
    #[serde(rename = "connected_peers", default)]
    pub connected_peers: usize,
    #[serde(rename = "active_channels", default)]
    pub active_channels: usize,
    #[serde(rename = "total_capacity", default)]
    pub total_capacity: u64,
}

/// Wrapper for list_channels RPC response (result contains {"channels": [...]}).
#[derive(Debug, Deserialize)]
struct ChannelsResponse {
    channels: Vec<ChannelInfo>,
}

/// Wrapper for list_peers RPC response (result contains {"peers": [...]}).
#[derive(Debug, Deserialize)]
struct PeersResponse {
    peers: Vec<PeerInfo>,
}

/// Channel information from the Fiber node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    #[serde(rename = "channel_id")]
    pub channel_id: String,
    #[serde(rename = "state")]
    pub state: ChannelState,
    #[serde(rename = "local_balance", default)]
    pub local_balance: u64,
    #[serde(rename = "remote_balance", default)]
    pub remote_balance: u64,
    #[serde(rename = "local_pubkey", default)]
    pub local_pubkey: String,
    #[serde(rename = "remote_pubkey", default)]
    pub remote_pubkey: String,
    #[serde(rename = "capacity", default)]
    pub capacity: u64,
}

/// Channel state information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelState {
    #[serde(rename = "state_name")]
    pub state_name: String,
}

/// Peer information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    #[serde(rename = "pubkey")]
    pub pubkey: String,
    #[serde(rename = "address", default)]
    pub address: String,
    #[serde(rename = "connected", default)]
    pub connected: bool,
}

/// Payment invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    #[serde(rename = "invoice_address")]
    pub invoice_address: String,
    #[serde(rename = "payment_hash", default)]
    pub payment_hash: String,
}

/// Payment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    #[serde(rename = "payment_hash")]
    pub payment_hash: String,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "failed_error", default)]
    pub failed_error: Option<String>,
}

/// Route information from the Fiber node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    #[serde(rename = "path", default)]
    pub path: Vec<String>,
    #[serde(rename = "channels", default)]
    pub channels: Vec<String>,
    #[serde(rename = "total_fee", default)]
    pub total_fee: u64,
    #[serde(rename = "max_amount", default)]
    pub max_amount: u64,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur when communicating with a Fiber node.
#[derive(Debug, Clone)]
pub enum FiberClientError {
    /// Network/HTTP error
    Network(String),
    /// Request timed out
    Timeout(String),
    /// Response couldn't be parsed
    ParseError(String),
    /// Circuit breaker is open (too many failures)
    CircuitOpen,
    /// JSON-RPC error from the Fiber node
    RpcError { code: i64, message: String },
    /// Server returned an error status
    ServerError { status: u16, body: String },
    /// All retries exhausted
    RetriesExhausted { last_error: String, attempts: u32 },
}

impl std::fmt::Display for FiberClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {}", e),
            Self::Timeout(e) => write!(f, "timeout: {}", e),
            Self::ParseError(e) => write!(f, "parse error: {}", e),
            Self::CircuitOpen => write!(f, "circuit breaker is open"),
            Self::RpcError { code, message } => {
                write!(f, "rpc error {}: {}", code, message)
            }
            Self::ServerError { status, body } => {
                write!(f, "server error {}: {}", status, body)
            }
            Self::RetriesExhausted { last_error, attempts } => {
                write!(
                    f,
                    "retries exhausted after {} attempts: {}",
                    attempts, last_error
                )
            }
        }
    }
}

impl std::error::Error for FiberClientError {}

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

/// Circuit breaker state: Closed (normal) → Open (failing) → HalfOpen (testing).
struct CircuitBreaker {
    failure_count: AtomicU64,
    is_open: AtomicBool,
    last_failure: AtomicU64, // Unix timestamp seconds
    threshold: u32,
    reset_duration: Duration,
}

impl CircuitBreaker {
    fn new(threshold: u32, reset_duration: Duration) -> Self {
        Self {
            failure_count: AtomicU64::new(0),
            is_open: AtomicBool::new(false),
            last_failure: AtomicU64::new(0),
            threshold,
            reset_duration,
        }
    }

    fn is_open(&self) -> bool {
        if !self.is_open.load(Ordering::Relaxed) {
            return false;
        }
        // Check if reset duration has passed → half-open
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = self.last_failure.load(Ordering::Relaxed);
        if now - last >= self.reset_duration.as_secs() {
            // Half-open: allow one request through
            self.is_open.store(false, Ordering::Relaxed);
            self.failure_count.store(0, Ordering::Relaxed);
            info!("circuit breaker: half-open, allowing test request");
            return false;
        }
        true
    }

    fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.is_open.store(false, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_failure.store(now, Ordering::Relaxed);
        if count >= self.threshold as u64 {
            self.is_open.store(true, Ordering::Relaxed);
            warn!(
                "circuit breaker: opened after {} consecutive failures",
                count
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Fiber Client
// ---------------------------------------------------------------------------

/// Client for interacting with a Fiber node via JSON-RPC.
///
/// Supports configurable timeouts, retry with exponential backoff,
/// and circuit breaker pattern for resilience.
pub struct FiberClient {
    config: FiberNodeConfig,
    http: reqwest::Client,
    circuit: CircuitBreaker,
    request_id: AtomicU64,
}

impl FiberClient {
    /// Create a new Fiber client with default configuration.
    pub fn new(config: FiberNodeConfig) -> Self {
        let timeout = config.request_timeout;
        let circuit = CircuitBreaker::new(
            config.circuit_breaker_threshold,
            config.circuit_breaker_reset,
        );

        let http = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build HTTP client");

        Self {
            config,
            http,
            circuit,
            request_id: AtomicU64::new(1),
        }
    }

    /// Create a Fiber client with a custom reqwest client (useful for testing).
    pub fn with_http_client(config: FiberNodeConfig, http: reqwest::Client) -> Self {
        let circuit = CircuitBreaker::new(
            config.circuit_breaker_threshold,
            config.circuit_breaker_reset,
        );
        Self {
            config,
            http,
            circuit,
            request_id: AtomicU64::new(1),
        }
    }

    /// Get the next request ID.
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Execute a JSON-RPC call with retry and circuit breaker logic.
    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<T, FiberClientError> {
        // Check circuit breaker
        if self.circuit.is_open() {
            warn!("{}: circuit breaker is open, rejecting request", method);
            return Err(FiberClientError::CircuitOpen);
        }

        let mut last_error = String::new();

        for attempt in 1..=self.config.max_retries {
            debug!("{}: attempt {}/{}", method, attempt, self.config.max_retries);

            let request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: self.next_id(),
                method: method.to_string(),
                params: params.clone(),
            };

            let body = serde_json::to_string(&request)
                .map_err(|e| FiberClientError::ParseError(e.to_string()))?;

            match self
                .http
                .post(&self.config.rpc_url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let text = resp
                            .text()
                            .await
                            .map_err(|e| FiberClientError::ParseError(e.to_string()))?;

                        let rpc_response: JsonRpcResponse = serde_json::from_str(&text)
                            .map_err(|e| FiberClientError::ParseError(e.to_string()))?;

                        if let Some(err) = rpc_response.error {
                            self.circuit.record_failure();
                            return Err(FiberClientError::RpcError {
                                code: err.code,
                                message: err.message,
                            });
                        }

                        if let Some(result) = rpc_response.result {
                            self.circuit.record_success();
                            return serde_json::from_value(result)
                                .map_err(|e| FiberClientError::ParseError(e.to_string()));
                        }

                        // Null result — return default
                        self.circuit.record_success();
                        return serde_json::from_value(serde_json::Value::Null)
                            .map_err(|e| FiberClientError::ParseError(e.to_string()));
                    }

                    // 4xx errors are not retryable (except 429)
                    if status.is_client_error() && status.as_u16() != 429 {
                        let body = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "<unreadable>".to_string());
                        self.circuit.record_failure();
                        return Err(FiberClientError::ServerError {
                            status: status.as_u16(),
                            body,
                        });
                    }

                    // 5xx or 429: retryable
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<unreadable>".to_string());
                    last_error = format!("server error {}: {}", status, body);
                    warn!(
                        "{}: attempt {} got status {}, retrying...",
                        method, attempt, status
                    );
                }
                Err(e) => {
                    last_error = e.to_string();
                    if e.is_timeout() {
                        warn!(
                            "{}: attempt {} timed out, retrying...",
                            method, attempt
                        );
                    } else if e.is_connect() {
                        warn!(
                            "{}: attempt {} connection failed, retrying...",
                            method, attempt
                        );
                    } else {
                        warn!(
                            "{}: attempt {} failed: {}, retrying...",
                            method, attempt, e
                        );
                    }
                }
            }

            // Exponential backoff with jitter
            if attempt < self.config.max_retries {
                let delay = self.config.retry_base_delay * 2u32.pow(attempt - 1)
                    + Duration::from_millis(rand_jitter(attempt));
                debug!("{}: sleeping {:?} before retry", method, delay);
                tokio::time::sleep(delay).await;
            }
        }

        self.circuit.record_failure();
        error!("{}: all {} retries exhausted", method, self.config.max_retries);
        Err(FiberClientError::RetriesExhausted {
            last_error,
            attempts: self.config.max_retries,
        })
    }

    // -- Node Info --

    /// Get Fiber node information.
    pub async fn node_info(&self) -> Result<NodeInfo, FiberClientError> {
        self.rpc_call("node_info", Some(serde_json::json!([]))).await
    }

    // -- Peer Management --

    /// Connect to a remote peer.
    pub async fn connect_peer(
        &self,
        pubkey: &str,
        address: &str,
        save: bool,
    ) -> Result<serde_json::Value, FiberClientError> {
        self.rpc_call(
            "connect_peer",
            Some(serde_json::json!([{
                "pubkey": pubkey,
                "address": address,
                "save": save
            }])),
        )
        .await
    }

    /// List connected peers.
    pub async fn list_peers(&self) -> Result<Vec<PeerInfo>, FiberClientError> {
        let resp: PeersResponse = self.rpc_call("list_peers", Some(serde_json::json!([{}]))).await?;
        Ok(resp.peers)
    }

    // -- Channel Management --

    /// Open a payment channel with a peer.
    pub async fn open_channel(
        &self,
        pubkey: &str,
        funding_amount: u64,
        public: bool,
    ) -> Result<serde_json::Value, FiberClientError> {
        // funding_amount in hex
        let funding_hex = format!("0x{:x}", funding_amount);
        self.rpc_call(
            "open_channel",
            Some(serde_json::json!([{
                "pubkey": pubkey,
                "funding_amount": funding_hex,
                "public": public
            }])),
        )
        .await
    }

    /// List all channels.
    pub async fn list_channels(
        &self,
        only_pending: bool,
    ) -> Result<Vec<ChannelInfo>, FiberClientError> {
        let resp: ChannelsResponse = self.rpc_call(
            "list_channels",
            Some(serde_json::json!([{"only_pending": only_pending}])),
        )
        .await?;
        Ok(resp.channels)
    }

    /// Shutdown (close) a channel.
    pub async fn shutdown_channel(
        &self,
        channel_id: &str,
    ) -> Result<serde_json::Value, FiberClientError> {
        self.rpc_call(
            "shutdown_channel",
            Some(serde_json::json!([{"channel_id": channel_id}])),
        )
        .await
    }

    // -- Payments --

    /// Create a payment invoice.
    pub async fn new_invoice(
        &self,
        amount: u64,
        currency: &str,
        description: &str,
    ) -> Result<Invoice, FiberClientError> {
        let amount_hex = format!("0x{:x}", amount);
        self.rpc_call(
            "new_invoice",
            Some(serde_json::json!([{
                "amount": amount_hex,
                "currency": currency,
                "description": description,
                "expiry": "0xe10"
            }])),
        )
        .await
    }

    /// Send a payment using an invoice.
    pub async fn send_payment(
        &self,
        invoice: &str,
    ) -> Result<PaymentResult, FiberClientError> {
        self.rpc_call(
            "send_payment",
            Some(serde_json::json!([{"invoice": invoice}])),
        )
        .await
    }

    /// Check payment status.
    pub async fn get_payment(
        &self,
        payment_hash: &str,
    ) -> Result<PaymentResult, FiberClientError> {
        self.rpc_call(
            "get_payment",
            Some(serde_json::json!([{"payment_hash": payment_hash}])),
        )
        .await
    }

    // -- Routing --

    /// Find a route through the Fiber network.
    pub async fn find_route(
        &self,
        amount: u64,
        currency: &str,
    ) -> Result<Option<RouteInfo>, FiberClientError> {
        let amount_hex = format!("0x{:x}", amount);
        self.rpc_call(
            "find_route",
            Some(serde_json::json!([{"amount": amount_hex, "currency": currency}])),
        )
        .await
        .map(Some)
        .or_else(|e| match e {
            FiberClientError::RpcError { code: -404, .. } => Ok(None),
            _ => Err(e),
        })
    }

    // -- Helpers --

    /// Check if the circuit breaker is currently open.
    pub fn is_circuit_open(&self) -> bool {
        self.circuit.is_open()
    }

    /// Get the configured RPC URL.
    pub fn rpc_url(&self) -> &str {
        &self.config.rpc_url
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Simple deterministic jitter based on attempt number (avoids needing `rand`).
fn rand_jitter(attempt: u32) -> u64 {
    // Simple hash of attempt number for jitter 0-100ms
    (attempt as u64).wrapping_mul(2654435761) % 100
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        assert!(!cb.is_open());

        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_open());

        cb.record_failure();
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());

        // After half-open period, should allow request and reset on success
        cb.is_open.store(false, Ordering::Relaxed); // simulate half-open
        cb.record_success();
        assert!(!cb.is_open());
        assert_eq!(cb.failure_count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_fiber_client_error_display() {
        let err = FiberClientError::CircuitOpen;
        assert!(err.to_string().contains("circuit breaker"));

        let err = FiberClientError::RetriesExhausted {
            last_error: "timeout".to_string(),
            attempts: 3,
        };
        assert!(err.to_string().contains("3"));
        assert!(err.to_string().contains("timeout"));

        let err = FiberClientError::RpcError {
            code: -32600,
            message: "Invalid Request".to_string(),
        };
        assert!(err.to_string().contains("rpc error"));
        assert!(err.to_string().contains("Invalid Request"));
    }

    #[test]
    fn test_fiber_client_default_config() {
        let config = FiberNodeConfig::default();
        assert_eq!(config.rpc_url, "http://127.0.0.1:8227");
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.circuit_breaker_threshold, 5);
    }

    #[test]
    fn test_rand_jitter_deterministic() {
        // Should be deterministic for same input
        assert_eq!(rand_jitter(1), rand_jitter(1));
        assert_eq!(rand_jitter(5), rand_jitter(5));
        // Should be < 100
        for i in 0..100 {
            assert!(rand_jitter(i) < 100);
        }
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "node_info".to_string(),
            params: Some(serde_json::json!([])),
        };
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"method\":\"node_info\""));
        assert!(serialized.contains("\"id\":1"));
    }

    #[test]
    fn test_channel_info_deserialization() {
        let json = r#"{
            "channel_id": "abc123",
            "state": {"state_name": "ChannelReady"},
            "local_balance": 50000,
            "remote_balance": 50000,
            "local_pubkey": "pub1",
            "remote_pubkey": "pub2",
            "capacity": 100000
        }"#;
        let channel: ChannelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(channel.channel_id, "abc123");
        assert_eq!(channel.state.state_name, "ChannelReady");
        assert_eq!(channel.local_balance, 50000);
    }

    #[test]
    fn test_node_info_deserialization() {
        let json = r#"{
            "pubkey": "03abc...",
            "version": "0.9.0",
            "listening_addr": "/ip4/127.0.0.1/tcp/8228",
            "connected_peers": 2,
            "active_channels": 1,
            "total_capacity": 1000000
        }"#;
        let info: NodeInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.pubkey, "03abc...");
        assert_eq!(info.version, "0.9.0");
        assert_eq!(info.connected_peers, 2);
    }

    #[test]
    fn test_invoice_deserialization() {
        let json = r#"{
            "invoice_address": "fibt100000000001p...",
            "payment_hash": "0xabc123"
        }"#;
        let invoice: Invoice = serde_json::from_str(json).unwrap();
        assert!(invoice.invoice_address.starts_with("fibt"));
    }

    #[test]
    fn test_payment_result_deserialization() {
        let json = r#"{
            "payment_hash": "0xabc123",
            "status": "Success",
            "failed_error": null
        }"#;
        let result: PaymentResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.status, "Success");
        assert!(result.failed_error.is_none());
    }
}
