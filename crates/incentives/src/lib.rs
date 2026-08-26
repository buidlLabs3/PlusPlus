//! Incentives — Fee distribution and reputation for intermediary routing nodes.
//!
//! Handles: fee calculation, distribution between maker/taker/intermediary,
//! protocol treasury cut, and reputation tracking.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Fee structure for a swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    /// Maker fee (basis points, e.g., 0 = free for makers)
    pub maker_fee_bps: u64,
    /// Taker fee (basis points, e.g., 20 = 0.2%)
    pub taker_fee_bps: u64,
    /// Protocol treasury cut (basis points, e.g., 5 = 0.05%)
    pub protocol_fee_bps: u64,
    /// Intermediary routing fee (basis points)
    pub routing_fee_bps: u64,
}

impl Default for FeeStructure {
    fn default() -> Self {
        Self {
            maker_fee_bps: 0,      // makers pay nothing
            taker_fee_bps: 20,     // takers pay 0.2%
            protocol_fee_bps: 5,   // protocol takes 0.05%
            routing_fee_bps: 15,   // intermediary gets 0.15%
        }
    }
}

/// Fee distribution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDistribution {
    /// Total swap amount
    pub total_amount: u64,
    /// Fee paid by maker
    pub maker_fee: u64,
    /// Fee paid by taker
    pub taker_fee: u64,
    /// Total fees collected
    pub total_fees: u64,
    /// Protocol treasury share
    pub protocol_share: u64,
    /// Intermediary share
    pub intermediary_share: u64,
    /// Number of intermediaries routing the swap
    pub intermediary_count: usize,
}

/// Reputation record for a routing node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationRecord {
    /// Node's lock hash
    pub node_lock: String,
    /// Total swaps routed
    pub swaps_routed: u64,
    /// Total fees earned
    pub fees_earned: u64,
    /// Successful routing rate (0-10000 basis points)
    pub success_rate_bps: u64,
    /// Uptime in blocks
    pub uptime_blocks: u64,
    /// Total downtime in blocks
    pub downtime_blocks: u64,
    /// Reputation score (composite, 0-10000)
    pub score: u64,
}

/// Node tier based on reputation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeTier {
    /// Score < 2000: new/unproven node
    Unproven,
    /// Score 2000-5000: basic routing node
    Basic,
    /// Score 5000-8000: trusted routing node
    Trusted,
    /// Score 8000+: preferred routing node
    Preferred,
}

// ---------------------------------------------------------------------------
// Fee operations
// ---------------------------------------------------------------------------

/// Calculate the fee distribution for a swap.
pub fn calculate_fees(
    amount: u64,
    fee_structure: &FeeStructure,
    intermediary_count: usize,
) -> FeeDistribution {
    let maker_fee = (amount * fee_structure.maker_fee_bps) / 10_000;
    let taker_fee = (amount * fee_structure.taker_fee_bps) / 10_000;
    let total_fees = maker_fee + taker_fee;

    let protocol_share = (total_fees * fee_structure.protocol_fee_bps) / 10_000;
    let routing_share = total_fees - protocol_share;

    // Split routing fee equally among intermediaries
    let intermediary_share = if intermediary_count > 0 {
        routing_share / intermediary_count as u64
    } else {
        0
    };

    FeeDistribution {
        total_amount: amount,
        maker_fee,
        taker_fee,
        total_fees,
        protocol_share,
        intermediary_share,
        intermediary_count,
    }
}

/// Calculate fee as a percentage of amount.
pub fn fee_percentage(amount: u64, fee_bps: u64) -> u64 {
    (amount * fee_bps) / 10_000
}

// ---------------------------------------------------------------------------
// Reputation operations
// ---------------------------------------------------------------------------

/// Create a new reputation record for a node.
pub fn create_reputation(node_lock: &str) -> ReputationRecord {
    ReputationRecord {
        node_lock: node_lock.to_string(),
        swaps_routed: 0,
        fees_earned: 0,
        success_rate_bps: 10_000, // start at 100%
        uptime_blocks: 0,
        downtime_blocks: 0,
        score: 1000, // starting score
    }
}

/// Update reputation after a successful routing.
pub fn record_success(record: &mut ReputationRecord, fee_earned: u64) {
    record.swaps_routed += 1;
    record.fees_earned += fee_earned;

    // Increase score slightly for each success (max 10000)
    let bonus = std::cmp::min(100, 10_000_u64.saturating_sub(record.score));
    record.score = std::cmp::min(10_000, record.score + bonus);
}

/// Update reputation after a failed routing.
pub fn record_failure(record: &mut ReputationRecord) {
    // Decrease score for failures
    let penalty = std::cmp::min(500, record.score);
    record.score = record.score.saturating_sub(penalty);
}

/// Update uptime tracking.
pub fn update_uptime(record: &mut ReputationRecord, blocks_online: u64, blocks_offline: u64) {
    record.uptime_blocks += blocks_online;
    record.downtime_blocks += blocks_offline;

    // Recalculate success rate
    let total = record.uptime_blocks + record.downtime_blocks;
    if total > 0 {
        record.success_rate_bps = (record.uptime_blocks * 10_000) / total;
    }
}

/// Get the tier for a reputation score.
pub fn get_tier(score: u64) -> NodeTier {
    if score >= 8000 {
        NodeTier::Preferred
    } else if score >= 5000 {
        NodeTier::Trusted
    } else if score >= 2000 {
        NodeTier::Basic
    } else {
        NodeTier::Unproven
    }
}

/// Compare two nodes by reputation score (higher is better).
pub fn compare_reputation(a: &ReputationRecord, b: &ReputationRecord) -> std::cmp::Ordering {
    b.score.cmp(&a.score)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fee_structure() {
        let fees = FeeStructure::default();
        assert_eq!(fees.maker_fee_bps, 0);
        assert_eq!(fees.taker_fee_bps, 20);
        assert_eq!(fees.protocol_fee_bps, 5);
        assert_eq!(fees.routing_fee_bps, 15);
    }

    #[test]
    fn test_calculate_fees() {
        let fees = FeeStructure::default();
        let dist = calculate_fees(1_000_000, &fees, 2);

        // Taker pays 0.2% = 2000
        assert_eq!(dist.taker_fee, 2000);
        // Maker pays 0%
        assert_eq!(dist.maker_fee, 0);
        // Total fees = 2000
        assert_eq!(dist.total_fees, 2000);
        // Protocol takes 0.05% of total = 1
        assert_eq!(dist.protocol_share, 1);
        // Intermediaries split the rest
        assert!(dist.intermediary_share > 0);
    }

    #[test]
    fn test_fee_percentage() {
        assert_eq!(fee_percentage(1_000_000, 20), 2000); // 0.2%
        assert_eq!(fee_percentage(1_000_000, 0), 0);
        assert_eq!(fee_percentage(1_000_000, 10000), 1_000_000); // 100%
    }

    #[test]
    fn test_reputation_create() {
        let rep = create_reputation("node_123");
        assert_eq!(rep.node_lock, "node_123");
        assert_eq!(rep.swaps_routed, 0);
        assert_eq!(rep.score, 1000);
    }

    #[test]
    fn test_reputation_success() {
        let mut rep = create_reputation("node_123");
        record_success(&mut rep, 100);
        assert_eq!(rep.swaps_routed, 1);
        assert_eq!(rep.fees_earned, 100);
        assert!(rep.score > 1000);
    }

    #[test]
    fn test_reputation_failure() {
        let mut rep = create_reputation("node_123");
        rep.score = 5000;
        record_failure(&mut rep);
        assert!(rep.score < 5000);
    }

    #[test]
    fn test_node_tiers() {
        assert_eq!(get_tier(9000), NodeTier::Preferred);
        assert_eq!(get_tier(6000), NodeTier::Trusted);
        assert_eq!(get_tier(3000), NodeTier::Basic);
        assert_eq!(get_tier(1000), NodeTier::Unproven);
    }

    #[test]
    fn test_uptime_tracking() {
        let mut rep = create_reputation("node_123");
        update_uptime(&mut rep, 900, 100);
        assert_eq!(rep.uptime_blocks, 900);
        assert_eq!(rep.downtime_blocks, 100);
        assert_eq!(rep.success_rate_bps, 9000); // 90%
    }

    #[test]
    fn test_compare_reputation() {
        let mut a = create_reputation("a");
        let mut b = create_reputation("b");
        record_success(&mut a, 100);
        record_success(&mut b, 100);
        record_success(&mut b, 100);
        record_success(&mut b, 100);
        record_success(&mut b, 100);

        // b has more successes, should have higher score
        assert!(b.score > a.score, "b.score ({}) should be > a.score ({})", b.score, a.score);
        // compare_reputation returns b.cmp(a), so when b > a it returns Greater
        assert_eq!(compare_reputation(&a, &b), std::cmp::Ordering::Greater);
    }
}
