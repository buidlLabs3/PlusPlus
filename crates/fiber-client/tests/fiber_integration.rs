//! Fiber Client Integration Tests
//!
//! These tests require running Fiber nodes (via docker-compose.local.yml).
//! Run with: cargo test -p fiber-client -- --ignored

use fiber_client::{FiberClient, FiberNodeConfig};
use std::time::Duration;

fn test_config(port: u16) -> FiberNodeConfig {
    FiberNodeConfig {
        rpc_url: format!("http://127.0.0.1:{}", port),
        request_timeout: Duration::from_secs(10),
        max_retries: 2,
        ..Default::default()
    }
}

async fn node1() -> FiberClient {
    FiberClient::new(test_config(8227))
}

async fn node2() -> FiberClient {
    FiberClient::new(test_config(8237))
}

async fn ensure_reachable(client: &FiberClient) -> bool {
    client.node_info().await.is_ok()
}

// ---------------------------------------------------------------------------
// Node Info
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore] // requires running Fiber nodes
async fn test_node_info_node1() {
    let client = node1().await;
    let info = client.node_info().await.expect("node1 should be reachable");
    assert!(!info.pubkey.is_empty(), "pubkey should not be empty");
    assert!(!info.version.is_empty(), "version should not be empty");
    println!("Node 1: pubkey={}, version={}", info.pubkey, info.version);
}

#[tokio::test]
#[ignore]
async fn test_node_info_node2() {
    let client = node2().await;
    let info = client.node_info().await.expect("node2 should be reachable");
    assert!(!info.pubkey.is_empty(), "pubkey should not be empty");
    println!("Node 2: pubkey={}, version={}", info.pubkey, info.version);
}

// ---------------------------------------------------------------------------
// Peer Management
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_list_peers() {
    let client = node1().await;
    if !ensure_reachable(&client).await {
        eprintln!("Skipping: node1 not reachable");
        return;
    }
    let peers = client.list_peers().await.expect("list_peers should succeed");
    println!("Node 1 has {} peers", peers.len());
    // After setup-local.sh, should have at least 1 peer
    for peer in &peers {
        println!("  Peer: {} connected={}", peer.pubkey, peer.connected);
    }
}

// ---------------------------------------------------------------------------
// Channel Management
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_list_channels() {
    let client = node1().await;
    if !ensure_reachable(&client).await {
        eprintln!("Skipping: node1 not reachable");
        return;
    }
    let channels = client.list_channels(false).await.expect("list_channels should succeed");
    println!("Node 1 has {} channels", channels.len());
    for ch in &channels {
        println!(
            "  Channel: {} state={} capacity={} local={} remote={}",
            ch.channel_id, ch.state.state_name, ch.capacity, ch.local_balance, ch.remote_balance
        );
    }
}

#[tokio::test]
#[ignore]
async fn test_open_channel() {
    let client1 = node1().await;
    let client2 = node2().await;

    if !ensure_reachable(&client1).await || !ensure_reachable(&client2).await {
        eprintln!("Skipping: nodes not reachable");
        return;
    }

    // Get node2's pubkey
    let info2 = client2.node_info().await.expect("node2 should be reachable");

    // Open channel from node1 to node2 with 100 CKB
    let result = client1
        .open_channel(&info2.pubkey, 100_000_000_000, true)
        .await;
    match result {
        Ok(val) => println!("Channel open result: {:?}", val),
        Err(e) => println!("Channel open failed (may already exist): {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn test_shutdown_channel() {
    let client = node1().await;
    if !ensure_reachable(&client).await {
        eprintln!("Skipping: node1 not reachable");
        return;
    }

    let channels = client.list_channels(false).await.expect("list_channels");
    if let Some(ch) = channels.first() {
        let result = client.shutdown_channel(&ch.channel_id).await;
        match result {
            Ok(val) => println!("Channel close result: {:?}", val),
            Err(e) => println!("Channel close failed: {}", e),
        }
    } else {
        eprintln!("No channels to close");
    }
}

// ---------------------------------------------------------------------------
// Invoice & Payment
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_create_invoice() {
    let client = node1().await;
    if !ensure_reachable(&client).await {
        eprintln!("Skipping: node1 not reachable");
        return;
    }

    let invoice = client
        .new_invoice(1_000_000, "CKB", "test payment")
        .await
        .expect("create invoice should succeed");

    assert!(!invoice.invoice_address.is_empty(), "invoice address should not be empty");
    println!("Invoice: address={}, hash={}", invoice.invoice_address, invoice.payment_hash);
}

#[tokio::test]
#[ignore]
async fn test_get_payment_status() {
    let client = node1().await;
    if !ensure_reachable(&client).await {
        eprintln!("Skipping: node1 not reachable");
        return;
    }

    // Check a non-existent payment hash
    let result = client.get_payment("0xnonexistent").await;
    match result {
        Ok(r) => println!("Payment status: {}", r.status),
        Err(e) => println!("Payment lookup error (expected): {}", e),
    }
}

// ---------------------------------------------------------------------------
// Route Finding
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_find_route() {
    let client = node1().await;
    if !ensure_reachable(&client).await {
        eprintln!("Skipping: node1 not reachable");
        return;
    }

    let result = client.find_route(1_000_000, "CKB").await;
    match result {
        Ok(Some(route)) => {
            println!("Route found:");
            println!("  Path: {:?}", route.path);
            println!("  Fee: {}", route.total_fee);
            println!("  Max: {}", route.max_amount);
        }
        Ok(None) => println!("No route found (expected if no channels)"),
        Err(e) => println!("Route finding error: {}", e),
    }
}

// ---------------------------------------------------------------------------
// Circuit Breaker
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_circuit_breaker() {
    // Use a port that's definitely not running a Fiber node
    let config = FiberNodeConfig {
        rpc_url: "http://127.0.0.1:19999".to_string(),
        request_timeout: Duration::from_secs(1),
        max_retries: 2,
        circuit_breaker_threshold: 3,
        ..Default::default()
    };
    let client = FiberClient::new(config);

    // Should fail and eventually open circuit breaker
    for i in 0..5 {
        let _ = client.node_info().await;
        println!(
            "Attempt {}: circuit_open={}",
            i + 1,
            client.is_circuit_open()
        );
    }

    assert!(client.is_circuit_open(), "circuit breaker should be open after 3 failures");
}
