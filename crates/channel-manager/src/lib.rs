//! Channel Manager — Manages Fiber channel lifecycle for atomic swaps.
//!
//! Handles: open, query, capacity check, rebalance, close, dispute resolution.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Status of a Fiber channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelStatus {
    Opening,
    Active,
    Closing,
    Closed,
    Disputed,
}

/// Represents a Fiber payment channel between two parties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Unique channel identifier
    pub channel_id: [u8; 32],
    /// Party A's lock hash
    pub party_a_lock: [u8; 32],
    /// Party B's lock hash
    pub party_b_lock: [u8; 32],
    /// Token type in this channel
    pub token_type: TokenInfo,
    /// Total capacity (in smallest unit)
    pub capacity: u64,
    /// Balance allocated to party A
    pub balance_a: u64,
    /// Balance allocated to party B
    pub balance_b: u64,
    /// Current status
    pub status: ChannelStatus,
    /// Block number when channel was opened
    pub opened_at: u64,
    /// Block number when channel was closed (0 if still open)
    pub closed_at: u64,
    /// Sequence number for state updates
    pub sequence: u64,
}

/// Token information for multi-token support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub code_hash: [u8; 32],
    pub hash_type: u8,
    pub args: Vec<u8>,
}

impl TokenInfo {
    /// Native CKB token
    pub fn ckb() -> Self {
        Self {
            code_hash: [0u8; 32],
            hash_type: 0,
            args: vec![],
        }
    }
}

/// Result of a channel operation.
#[derive(Debug)]
pub struct ChannelResult {
    pub success: bool,
    pub channel_id: Option<[u8; 32]>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Channel operations
// ---------------------------------------------------------------------------

/// Open a new Fiber channel with a counterparty.
pub fn open_channel(
    party_a: [u8; 32],
    party_b: [u8; 32],
    token: TokenInfo,
    capacity: u64,
    current_block: u64,
) -> ChannelResult {
    if capacity == 0 {
        return ChannelResult {
            success: false,
            channel_id: None,
            error: Some("capacity must be > 0".to_string()),
        };
    }

    if party_a == party_b {
        return ChannelResult {
            success: false,
            channel_id: None,
            error: Some("cannot open channel with yourself".to_string()),
        };
    }

    // Generate channel ID from inputs
    let mut blake2b = blake2b_rs::Blake2bBuilder::new(32).build();
    blake2b.update(&party_a);
    blake2b.update(&party_b);
    blake2b.update(&token.code_hash);
    blake2b.update(&capacity.to_le_bytes());
    blake2b.update(&current_block.to_le_bytes());
    let mut channel_id = [0u8; 32];
    blake2b.finalize(&mut channel_id);

    let channel = Channel {
        channel_id,
        party_a_lock: party_a,
        party_b_lock: party_b,
        token_type: token,
        capacity,
        balance_a: capacity / 2,
        balance_b: capacity - capacity / 2,
        status: ChannelStatus::Opening,
        opened_at: current_block,
        closed_at: 0,
        sequence: 0,
    };

    // In production, this would submit an on-chain transaction
    // For now, return the channel info
    println!("Channel {} opened: {} capacity", hex::encode(channel_id), channel.capacity);

    ChannelResult {
        success: true,
        channel_id: Some(channel_id),
        error: None,
    }
}

/// Check if a channel has sufficient capacity for a given amount.
pub fn check_capacity(channel: &Channel, amount: u64, from_party: &[u8; 32]) -> bool {
    if channel.status != ChannelStatus::Active {
        return false;
    }

    if *from_party == channel.party_a_lock {
        channel.balance_a >= amount
    } else if *from_party == channel.party_b_lock {
        channel.balance_b >= amount
    } else {
        false
    }
}

/// Update channel balance after a successful swap.
pub fn update_balance(
    channel: &mut Channel,
    amount: u64,
    from_party: &[u8; 32],
) -> Result<(), String> {
    if channel.status != ChannelStatus::Active {
        return Err("channel not active".to_string());
    }

    if !check_capacity(channel, amount, from_party) {
        return Err("insufficient balance".to_string());
    }

    if *from_party == channel.party_a_lock {
        channel.balance_a -= amount;
        channel.balance_b += amount;
    } else if *from_party == channel.party_b_lock {
        channel.balance_b -= amount;
        channel.balance_a += amount;
    } else {
        return Err("unknown party".to_string());
    }

    channel.sequence += 1;
    Ok(())
}

/// Initiate channel close.
pub fn close_channel(channel: &mut Channel, current_block: u64) -> Result<(), String> {
    if channel.status != ChannelStatus::Active {
        return Err("channel not active".to_string());
    }

    channel.status = ChannelStatus::Closing;
    channel.closed_at = current_block;
    Ok(())
}

/// Force close a disputed channel.
pub fn force_close(channel: &mut Channel) -> Result<(), String> {
    if channel.status != ChannelStatus::Active && channel.status != ChannelStatus::Disputed {
        return Err("channel cannot be force-closed".to_string());
    }

    channel.status = ChannelStatus::Closed;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_lock(n: u8) -> [u8; 32] {
        let mut lock = [0u8; 32];
        lock[0] = n;
        lock
    }

    #[test]
    fn test_open_channel() {
        let a = dummy_lock(1);
        let b = dummy_lock(2);
        let token = TokenInfo::ckb();

        let result = open_channel(a, b, token, 1_000_000_000, 100);
        assert!(result.success);
        assert!(result.channel_id.is_some());
    }

    #[test]
    fn test_open_channel_zero_capacity() {
        let a = dummy_lock(1);
        let b = dummy_lock(2);
        let token = TokenInfo::ckb();

        let result = open_channel(a, b, token, 0, 100);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("capacity"));
    }

    #[test]
    fn test_open_channel_same_party() {
        let a = dummy_lock(1);
        let token = TokenInfo::ckb();

        let result = open_channel(a, a, token, 1_000_000_000, 100);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("yourself"));
    }

    #[test]
    fn test_check_capacity() {
        let a = dummy_lock(1);
        let b = dummy_lock(2);
        let token = TokenInfo::ckb();

        let mut channel = Channel {
            channel_id: [0u8; 32],
            party_a_lock: a,
            party_b_lock: b,
            token_type: token,
            capacity: 1_000_000_000,
            balance_a: 500_000_000,
            balance_b: 500_000_000,
            status: ChannelStatus::Active,
            opened_at: 100,
            closed_at: 0,
            sequence: 0,
        };

        assert!(check_capacity(&channel, 500_000_000, &a));
        assert!(!check_capacity(&channel, 500_000_001, &a));
        assert!(check_capacity(&channel, 1, &b));
    }

    #[test]
    fn test_update_balance() {
        let a = dummy_lock(1);
        let b = dummy_lock(2);
        let token = TokenInfo::ckb();

        let mut channel = Channel {
            channel_id: [0u8; 32],
            party_a_lock: a,
            party_b_lock: b,
            token_type: token,
            capacity: 1_000_000_000,
            balance_a: 500_000_000,
            balance_b: 500_000_000,
            status: ChannelStatus::Active,
            opened_at: 100,
            closed_at: 0,
            sequence: 0,
        };

        let result = update_balance(&mut channel, 100_000_000, &a);
        assert!(result.is_ok());
        assert_eq!(channel.balance_a, 400_000_000);
        assert_eq!(channel.balance_b, 600_000_000);
        assert_eq!(channel.sequence, 1);
    }

    #[test]
    fn test_close_channel() {
        let a = dummy_lock(1);
        let b = dummy_lock(2);
        let token = TokenInfo::ckb();

        let mut channel = Channel {
            channel_id: [0u8; 32],
            party_a_lock: a,
            party_b_lock: b,
            token_type: token,
            capacity: 1_000_000_000,
            balance_a: 500_000_000,
            balance_b: 500_000_000,
            status: ChannelStatus::Active,
            opened_at: 100,
            closed_at: 0,
            sequence: 0,
        };

        let result = close_channel(&mut channel, 200);
        assert!(result.is_ok());
        assert_eq!(channel.status, ChannelStatus::Closing);
        assert_eq!(channel.closed_at, 200);
    }
}
