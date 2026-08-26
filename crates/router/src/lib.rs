//! Router — Finds optimal paths through Fiber network for atomic swaps.
//!
//! Uses Dijkstra's algorithm with fee-weighted and liquidity-weighted edges.
//! For large amounts that exceed single-channel capacity, splits across multiple
//! paths (multi-path routing).

use channel_manager::{Channel, TokenInfo};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A node in the Fiber network (identified by lock hash).
pub type NodeId = [u8; 32];

/// An edge in the routing graph (a channel between two nodes).
#[derive(Debug, Clone)]
pub struct Edge {
    pub channel_id: [u8; 32],
    pub from: NodeId,
    pub to: NodeId,
    pub capacity: u64,
    pub fee_rate: u64, // fee per unit (basis points, e.g., 10 = 0.1%)
    pub token: TokenInfo,
}

/// A route through the Fiber network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Nodes in order: source → ... → destination
    pub path: Vec<NodeId>,
    /// Channel IDs used in order
    pub channels: Vec<[u8; 32]>,
    /// Total fee for this route
    pub total_fee: u64,
    /// Amount routed through this path
    pub amount: u64,
    /// Bottleneck capacity of the weakest link
    pub bottleneck: u64,
}

/// A multi-path result splitting a large amount across routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPath {
    /// Individual routes (each carries a portion of the total)
    pub routes: Vec<Route>,
    /// Total amount across all routes
    pub total_amount: u64,
    /// Total fee across all routes
    pub total_fee: u64,
    /// Whether the full amount was successfully routed
    pub fully_routed: bool,
}

/// The Fiber network graph.
#[derive(Debug, Clone)]
pub struct NetworkGraph {
    /// All edges (channels)
    edges: Vec<Edge>,
    /// Adjacency list: node → list of edge indices
    adjacency: Vec<(NodeId, Vec<usize>)>,
}

/// Result of a routing attempt.
#[derive(Debug)]
pub struct RouteResult {
    pub success: bool,
    pub route: Option<Route>,
    pub error: Option<String>,
}

/// Result of a multi-path routing attempt.
#[derive(Debug)]
pub struct MultiPathResult {
    pub success: bool,
    pub multipath: Option<MultiPath>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Network graph
// ---------------------------------------------------------------------------

impl NetworkGraph {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            adjacency: Vec::new(),
        }
    }

    /// Add a channel as an edge in both directions.
    pub fn add_channel(&mut self, channel: &Channel, fee_rate: u64) {
        let edge_a_to_b = Edge {
            channel_id: channel.channel_id,
            from: channel.party_a_lock,
            to: channel.party_b_lock,
            capacity: channel.balance_a,
            fee_rate,
            token: channel.token_type.clone(),
        };

        let edge_b_to_a = Edge {
            channel_id: channel.channel_id,
            from: channel.party_b_lock,
            to: channel.party_a_lock,
            capacity: channel.balance_b,
            fee_rate,
            token: channel.token_type.clone(),
        };

        let idx_a = self.edges.len();
        self.edges.push(edge_a_to_b);
        let idx_b = self.edges.len();
        self.edges.push(edge_b_to_a);

        // Update adjacency lists
        self.add_adjacency(channel.party_a_lock, idx_a);
        self.add_adjacency(channel.party_b_lock, idx_b);
    }

    fn add_adjacency(&mut self, node: NodeId, edge_idx: usize) {
        if let Some((_, indices)) = self.adjacency.iter_mut().find(|(n, _)| *n == node) {
            indices.push(edge_idx);
        } else {
            self.adjacency.push((node, vec![edge_idx]));
        }
    }

    /// Get neighbors of a node with available capacity >= min_capacity.
    fn neighbors(&self, node: &NodeId, min_capacity: u64) -> Vec<(&Edge, u64)> {
        self.adjacency
            .iter()
            .find(|(n, _)| n == node)
            .map(|(_, indices)| {
                indices
                    .iter()
                    .filter_map(|&idx| {
                        let edge = &self.edges[idx];
                        if edge.from == *node && edge.capacity >= min_capacity {
                            Some((edge, edge.fee_rate))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    /// Get all neighbors regardless of capacity (for multi-path exploration).
    fn all_neighbors(&self, node: &NodeId) -> Vec<&Edge> {
        self.adjacency
            .iter()
            .find(|(n, _)| n == node)
            .map(|(_, indices)| {
                indices
                    .iter()
                    .filter_map(|&idx| {
                        let edge = &self.edges[idx];
                        if edge.from == *node && edge.capacity > 0 {
                            Some(edge)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    /// Get the edge between two specific nodes, if it exists.
    fn edge_between(&self, from: &NodeId, to: &NodeId) -> Option<&Edge> {
        self.edges
            .iter()
            .find(|e| e.from == *from && e.to == *to && e.capacity > 0)
    }

    /// Temporarily consume capacity along a path (for multi-path splitting).
    fn consume_capacity(&mut self, path: &[NodeId], amount: u64) {
        for window in path.windows(2) {
            if let Some(edge) = self
                .edges
                .iter_mut()
                .find(|e| e.from == window[0] && e.to == window[1])
            {
                edge.capacity = edge.capacity.saturating_sub(amount);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Routing (Dijkstra with liquidity awareness)
// ---------------------------------------------------------------------------

/// Find the cheapest route from source to destination.
///
/// The cost function combines fee and a liquidity penalty: routes through
/// channels with higher capacity relative to the amount are preferred,
/// so the algorithm avoids bottlenecks.
pub fn find_route(
    graph: &NetworkGraph,
    source: &NodeId,
    destination: &NodeId,
    amount: u64,
    max_hops: usize,
) -> RouteResult {
    if source == destination {
        return RouteResult {
            success: false,
            route: None,
            error: Some("source and destination are the same".to_string()),
        };
    }

    if amount == 0 {
        return RouteResult {
            success: false,
            route: None,
            error: Some("amount must be > 0".to_string()),
        };
    }

    // Dijkstra's algorithm
    // State: (cost, node, hops, bottleneck_so_far)
    // prev maps: node → (came_from_node, channel_id)
    let mut prev: std::collections::HashMap<NodeId, (NodeId, [u8; 32])> =
        std::collections::HashMap::new();
    // best_cost: node → (cost, bottleneck)
    let mut best_state: std::collections::HashMap<NodeId, (u64, u64)> =
        std::collections::HashMap::new();
    // (cost, node, hops, bottleneck)
    let mut heap: BinaryHeap<Reverse<(u64, NodeId, usize, u64)>> = BinaryHeap::new();

    best_state.insert(*source, (0, u64::MAX));
    heap.push(Reverse((0, *source, 0, u64::MAX)));

    while let Some(Reverse((cost, current, hops, bottleneck))) = heap.pop() {
        // Skip if we already found a better path to this node
        if let Some(&(best_cost, best_bottleneck)) = best_state.get(&current) {
            // A state is dominated if it has both higher cost AND lower (or equal) bottleneck
            if cost > best_cost && bottleneck <= best_bottleneck {
                continue;
            }
            // If same cost but lower bottleneck, skip
            if cost >= best_cost && bottleneck < best_bottleneck {
                continue;
            }
        }

        if current == *destination {
            // Reconstruct path
            let mut path = vec![current];
            let mut channels = vec![];
            let mut node = current;
            while let Some(&(prev_node, ref channel_id)) = prev.get(&node) {
                path.insert(0, prev_node);
                channels.insert(0, *channel_id);
                node = prev_node;
            }

            return RouteResult {
                success: true,
                route: Some(Route {
                    path,
                    channels,
                    total_fee: cost,
                    amount,
                    bottleneck,
                }),
                error: None,
            };
        }

        if hops >= max_hops {
            continue;
        }

        // Explore neighbors
        for (edge, _fee_rate) in graph.neighbors(&current, amount) {
            let fee = (amount * edge.fee_rate) / 10_000;
            let new_cost = cost + fee;
            let new_bottleneck = bottleneck.min(edge.capacity);

            let dominated = best_state
                .get(&edge.to)
                .map_or(false, |&(c, b)| c <= new_cost && b >= new_bottleneck);
            if !dominated {
                best_state.insert(edge.to, (new_cost, new_bottleneck));
                prev.insert(edge.to, (current, edge.channel_id));
                heap.push(Reverse((new_cost, edge.to, hops + 1, new_bottleneck)));
            }
        }
    }

    RouteResult {
        success: false,
        route: None,
        error: Some("no route found".to_string()),
    }
}

/// Find the cheapest route from source to destination with a custom minimum
/// capacity per edge. Used by multi-path routing after capacity has been
/// partially consumed by earlier paths.
fn find_route_with_min_capacity(
    graph: &NetworkGraph,
    source: &NodeId,
    destination: &NodeId,
    amount: u64,
    max_hops: usize,
    min_edge_capacity: u64,
) -> RouteResult {
    if source == destination || amount == 0 {
        return RouteResult {
            success: false,
            route: None,
            error: Some("invalid parameters".to_string()),
        };
    }

    let mut prev: std::collections::HashMap<NodeId, (NodeId, [u8; 32])> =
        std::collections::HashMap::new();
    let mut best_state: std::collections::HashMap<NodeId, (u64, u64)> =
        std::collections::HashMap::new();
    let mut heap: BinaryHeap<Reverse<(u64, NodeId, usize, u64)>> = BinaryHeap::new();

    best_state.insert(*source, (0, u64::MAX));
    heap.push(Reverse((0, *source, 0, u64::MAX)));

    while let Some(Reverse((cost, current, hops, bottleneck))) = heap.pop() {
        if let Some(&(best_cost, best_bottleneck)) = best_state.get(&current) {
            if cost > best_cost && bottleneck <= best_bottleneck {
                continue;
            }
            if cost >= best_cost && bottleneck < best_bottleneck {
                continue;
            }
        }

        if current == *destination {
            let mut path = vec![current];
            let mut channels = vec![];
            let mut node = current;
            while let Some(&(prev_node, ref channel_id)) = prev.get(&node) {
                path.insert(0, prev_node);
                channels.insert(0, *channel_id);
                node = prev_node;
            }
            return RouteResult {
                success: true,
                route: Some(Route {
                    path,
                    channels,
                    total_fee: cost,
                    amount,
                    bottleneck,
                }),
                error: None,
            };
        }

        if hops >= max_hops {
            continue;
        }

        // Use custom min capacity filter
        for (edge, _) in graph.neighbors(&current, min_edge_capacity) {
            let fee = (amount * edge.fee_rate) / 10_000;
            let new_cost = cost + fee;
            let new_bottleneck = bottleneck.min(edge.capacity);

            let dominated = best_state
                .get(&edge.to)
                .map_or(false, |&(c, b)| c <= new_cost && b >= new_bottleneck);
            if !dominated {
                best_state.insert(edge.to, (new_cost, new_bottleneck));
                prev.insert(edge.to, (current, edge.channel_id));
                heap.push(Reverse((new_cost, edge.to, hops + 1, new_bottleneck)));
            }
        }
    }

    RouteResult {
        success: false,
        route: None,
        error: Some("no route found".to_string()),
    }
}

/// Find multiple routes from source to destination, each carrying up to
/// the bottleneck capacity of that path. Uses iterative routing with capacity
/// removal to find disjoint-ish paths.
pub fn find_multi_path(
    graph: &NetworkGraph,
    source: &NodeId,
    destination: &NodeId,
    amount: u64,
    max_hops: usize,
    max_routes: usize,
) -> MultiPathResult {
    if source == destination {
        return MultiPathResult {
            success: false,
            multipath: None,
            error: Some("source and destination are the same".to_string()),
        };
    }

    if amount == 0 {
        return MultiPathResult {
            success: false,
            multipath: None,
            error: Some("amount must be > 0".to_string()),
        };
    }

    // If amount fits in a single route, just do single-path
    if max_routes <= 1 {
        let result = find_route(graph, source, destination, amount, max_hops);
        if result.success {
            let route = result.route.unwrap();
            return MultiPathResult {
                success: true,
                multipath: Some(MultiPath {
                    total_amount: route.amount,
                    total_fee: route.total_fee,
                    fully_routed: route.amount >= amount,
                    routes: vec![route],
                }),
                error: None,
            };
        }
        return MultiPathResult {
            success: false,
            multipath: None,
            error: result.error,
        };
    }

    // Multi-path: iteratively find routes and consume capacity
    let mut g = graph.clone();
    let mut routes = Vec::new();
    let mut remaining = amount;

    for _ in 0..max_routes {
        if remaining == 0 {
            break;
        }

        // Try to find a route with at least 1 unit of capacity available
        let route_result = find_route_with_min_capacity(
            &g, source, destination, remaining, max_hops, 1,
        );
        if !route_result.success {
            break;
        }

        let route = route_result.route.unwrap();
        // Route as much as the bottleneck allows, but not more than remaining
        let routed = remaining.min(route.bottleneck);
        if routed == 0 {
            break;
        }

        // Create a route with the actual routed amount
        let fee_ratio = route.total_fee as f64 / route.amount.max(1) as f64;
        let actual_fee = (routed as f64 * fee_ratio) as u64;

        let routed_route = Route {
            path: route.path,
            channels: route.channels,
            total_fee: actual_fee,
            amount: routed,
            bottleneck: route.bottleneck,
        };

        // Consume capacity along this path
        g.consume_capacity(&routed_route.path, routed);

        remaining = remaining.saturating_sub(routed);
        routes.push(routed_route);
    }

    let total_routed: u64 = routes.iter().map(|r| r.amount).sum();
    let total_fee: u64 = routes.iter().map(|r| r.total_fee).sum();

    if routes.is_empty() {
        return MultiPathResult {
            success: false,
            multipath: None,
            error: Some("no route found".to_string()),
        };
    }

    MultiPathResult {
        success: true,
        multipath: Some(MultiPath {
            routes,
            total_amount: total_routed,
            total_fee,
            fully_routed: total_routed >= amount,
        }),
        error: None,
    }
}

/// Calculate fee for a route.
pub fn calculate_fee(route: &Route, amount: u64) -> u64 {
    (amount * route.total_fee) / route.amount.max(1)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use channel_manager::{Channel, ChannelStatus};

    fn dummy_lock(n: u8) -> [u8; 32] {
        let mut lock = [0u8; 32];
        lock[0] = n;
        lock
    }

    fn make_channel(a: u8, b: u8, capacity: u64) -> Channel {
        Channel {
            channel_id: [a + b; 32],
            party_a_lock: dummy_lock(a),
            party_b_lock: dummy_lock(b),
            token_type: TokenInfo::ckb(),
            capacity,
            balance_a: capacity / 2,
            balance_b: capacity / 2,
            status: ChannelStatus::Active,
            opened_at: 100,
            closed_at: 0,
            sequence: 0,
        }
    }

    #[test]
    fn test_direct_route() {
        let mut graph = NetworkGraph::new();
        let ch = make_channel(1, 2, 1_000_000);
        graph.add_channel(&ch, 10); // 0.1% fee

        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_route(&graph, &source, &dest, 100_000, 3);
        assert!(result.success);
        let route = result.route.unwrap();
        assert_eq!(route.path.len(), 2);
        assert_eq!(route.total_fee, 100); // 100_000 * 10 / 10_000 = 100
        assert_eq!(route.bottleneck, 500_000); // balance_a = capacity/2
        assert_eq!(route.amount, 100_000);
    }

    #[test]
    fn test_multi_hop_route() {
        let mut graph = NetworkGraph::new();
        // 1 → 2 → 3
        let ch1 = make_channel(1, 2, 1_000_000);
        let ch2 = make_channel(2, 3, 1_000_000);
        graph.add_channel(&ch1, 10);
        graph.add_channel(&ch2, 10);

        let source = dummy_lock(1);
        let dest = dummy_lock(3);

        let result = find_route(&graph, &source, &dest, 100_000, 3);
        assert!(result.success);
        let route = result.route.unwrap();
        assert_eq!(route.path.len(), 3);
        assert!(route.total_fee > 0);
    }

    #[test]
    fn test_no_route() {
        let graph = NetworkGraph::new();

        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_route(&graph, &source, &dest, 100_000, 3);
        assert!(!result.success);
    }

    #[test]
    fn test_insufficient_capacity() {
        let mut graph = NetworkGraph::new();
        let ch = make_channel(1, 2, 100); // very low capacity
        graph.add_channel(&ch, 10);

        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_route(&graph, &source, &dest, 1_000_000, 3); // amount > capacity
        assert!(!result.success);
    }

    #[test]
    fn test_same_source_dest() {
        let graph = NetworkGraph::new();
        let source = dummy_lock(1);

        let result = find_route(&graph, &source, &source, 100_000, 3);
        assert!(!result.success);
    }

    #[test]
    fn test_zero_amount() {
        let graph = NetworkGraph::new();
        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_route(&graph, &source, &dest, 0, 3);
        assert!(!result.success);
    }

    #[test]
    fn test_liquidity_prefers_higher_capacity() {
        // Two paths: 1→2→4 (low capacity) and 1→3→4 (high capacity)
        // Both have same fee rate, but the high-capacity path should be preferred
        // because the liquidity-aware algorithm considers bottleneck
        let mut graph = NetworkGraph::new();

        // Path A: 1→2→4, capacity 200
        let ch_a1 = make_channel(1, 2, 200);
        let ch_a2 = make_channel(2, 4, 200);

        // Path B: 1→3→4, capacity 10_000
        let ch_b1 = make_channel(1, 3, 10_000);
        let ch_b2 = make_channel(3, 4, 10_000);

        graph.add_channel(&ch_a1, 10);
        graph.add_channel(&ch_a2, 10);
        graph.add_channel(&ch_b1, 10);
        graph.add_channel(&ch_b2, 10);

        let source = dummy_lock(1);
        let dest = dummy_lock(4);

        // Amount that fits in both paths (each has balance = capacity/2 = 100 or 5000)
        // But the high-cap path should be preferred for better bottleneck
        let result = find_route(&graph, &source, &dest, 100, 3);
        assert!(result.success);
        let route = result.route.unwrap();
        // The route through 3 should have higher bottleneck
        // Path B bottleneck = min(5000, 5000) = 5000, Path A bottleneck = min(100, 100) = 100
        assert_eq!(route.bottleneck, 5000);
    }

    #[test]
    fn test_find_multi_path_splits_large_amount() {
        let mut graph = NetworkGraph::new();

        // Two parallel paths: 1→2→4 and 1→3→4
        // Each has capacity 1000 (balance_a = 500 each direction)
        let ch1 = make_channel(1, 2, 1000);
        let ch2 = make_channel(2, 4, 1000);
        let ch3 = make_channel(1, 3, 1000);
        let ch4 = make_channel(3, 4, 1000);

        graph.add_channel(&ch1, 100); // 1% fee
        graph.add_channel(&ch2, 100);
        graph.add_channel(&ch3, 100);
        graph.add_channel(&ch4, 100);

        let source = dummy_lock(1);
        let dest = dummy_lock(4);

        // Request 800 — more than single channel can carry (500), but two paths can
        let result = find_multi_path(&graph, &source, &dest, 800, 3, 4);
        assert!(result.success);
        let mp = result.multipath.unwrap();
        assert!(mp.routes.len() >= 2);
        assert!(mp.total_amount >= 800);
        assert!(mp.total_fee > 0); // 800 * 100 / 10000 = 8 per hop
        assert!(mp.fully_routed);
    }

    #[test]
    fn test_find_multi_path_single_route_enough() {
        let mut graph = NetworkGraph::new();
        let ch = make_channel(1, 2, 10_000);
        graph.add_channel(&ch, 10);

        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_multi_path(&graph, &source, &dest, 100, 3, 4);
        assert!(result.success);
        let mp = result.multipath.unwrap();
        assert_eq!(mp.routes.len(), 1);
        assert!(mp.fully_routed);
    }

    #[test]
    fn test_find_multi_path_no_route() {
        let graph = NetworkGraph::new();
        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_multi_path(&graph, &source, &dest, 1000, 3, 4);
        assert!(!result.success);
    }

    #[test]
    fn test_find_multi_path_same_source_dest() {
        let graph = NetworkGraph::new();
        let source = dummy_lock(1);

        let result = find_multi_path(&graph, &source, &source, 1000, 3, 4);
        assert!(!result.success);
    }

    #[test]
    fn test_find_multi_path_partial_routable() {
        let mut graph = NetworkGraph::new();

        // Single path with limited capacity
        let ch = make_channel(1, 2, 100); // balance = 50
        graph.add_channel(&ch, 10);

        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        // Request 1000 but only 50 available
        let result = find_multi_path(&graph, &source, &dest, 1000, 3, 4);
        assert!(result.success);
        let mp = result.multipath.unwrap();
        assert!(!mp.fully_routed);
        assert!(mp.total_amount < 1000);
    }

    #[test]
    fn test_find_multi_path_zero_amount() {
        let graph = NetworkGraph::new();
        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_multi_path(&graph, &source, &dest, 0, 3, 4);
        assert!(!result.success);
    }

    #[test]
    fn test_find_multi_path_max_routes_one() {
        let mut graph = NetworkGraph::new();
        let ch = make_channel(1, 2, 10_000);
        graph.add_channel(&ch, 10);

        let source = dummy_lock(1);
        let dest = dummy_lock(2);

        let result = find_multi_path(&graph, &source, &dest, 100, 3, 1);
        assert!(result.success);
        let mp = result.multipath.unwrap();
        assert_eq!(mp.routes.len(), 1);
    }

    #[test]
    fn test_calculate_fee() {
        let route = Route {
            path: vec![dummy_lock(1), dummy_lock(2)],
            channels: vec![[3; 32]],
            total_fee: 200,
            amount: 10_000,
            bottleneck: 5000,
        };
        // Fee scales with amount
        let fee = calculate_fee(&route, 5000);
        assert_eq!(fee, 100); // 200 * 5000 / 10000
    }
}
