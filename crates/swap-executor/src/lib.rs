//! Swap Executor — Takes an accepted offer and builds the atomic swap transaction.
//!
//! The executor:
//! 1. Takes a signed offer from the seller
//! 2. Takes a signed acceptance from the buyer
//! 3. Builds a CKB transaction with the swap covenant as type script
//! 4. Gathers the required cells
//! 5. Submits to Fiber network for routing

use offer_protocol::{Offer, CellType};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A buyer's acceptance of an offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapAcceptance {
    /// The offer being accepted
    pub offer_id: [u8; 32],
    /// Buyer's lock script hash
    pub buyer_lock_hash: [u8; 32],
    /// Amount buyer wants to swap (must match offer.buy_amount or less)
    pub amount: u64,
    /// Buyer's signature
    pub signature: Vec<u8>,
}

/// Represents a cell to be used as input or output in the swap transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapCell {
    /// Cell type (code_hash, hash_type, args)
    pub cell_type: CellType,
    /// Lock script hash
    pub lock_hash: [u8; 32],
    /// Capacity (in shannons)
    pub capacity: u64,
    /// Data (for swap covenant: seller_lock_hash, buyer_lock_hash, amounts, expiry)
    pub data: Vec<u8>,
}

/// The result of building a swap transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapTransaction {
    /// Inputs: [seller_rgbpp_cell, buyer_btc_cell]
    pub inputs: Vec<SwapCell>,
    /// Outputs: [buyer_gets_rgbpp, seller_gets_btc]
    pub outputs: Vec<SwapCell>,
    /// The swap covenant type script
    pub covenant_type: CellType,
    /// Witness data (signatures)
    pub witnesses: Vec<Vec<u8>>,
    /// Cell deps (type scripts needed for verification)
    pub cell_deps: Vec<CellType>,
    /// Header deps (block headers for verification)
    pub header_deps: Vec<[u8; 32]>,
}

/// Status of a swap execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwapStatus {
    /// Offer accepted, building transaction
    Pending,
    /// Transaction submitted to Fiber
    Submitted,
    /// Transaction confirmed on-chain
    Confirmed,
    /// Swap failed (routing failure, insufficient capacity, etc.)
    Failed(String),
    /// Offer expired before execution
    Expired,
}

/// The result of executing a swap.
#[derive(Debug)]
pub struct SwapResult {
    pub status: SwapStatus,
    pub tx_hash: Option<[u8; 32]>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Swap execution
// ---------------------------------------------------------------------------

/// Build the swap cell data for the covenant.
///
/// Layout: [seller_lock_hash: 32] [buyer_lock_hash: 32] [sell_amount: 8] [buy_amount: 8] [expiry: 8]
pub fn build_swap_cell_data(
    seller_lock_hash: &[u8; 32],
    buyer_lock_hash: &[u8; 32],
    sell_amount: u64,
    buy_amount: u64,
    expiry: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(88);
    data.extend_from_slice(seller_lock_hash);
    data.extend_from_slice(buyer_lock_hash);
    data.extend_from_slice(&sell_amount.to_le_bytes());
    data.extend_from_slice(&buy_amount.to_le_bytes());
    data.extend_from_slice(&expiry.to_le_bytes());
    data
}

/// Build the swap transaction from an offer and acceptance.
///
/// This constructs the full transaction structure:
/// - Input 0: seller's RGB++ cell (the asset being sold)
/// - Input 1: buyer's BTC cell (the payment)
/// - Output 0: buyer receives RGB++ asset
/// - Output 1: seller receives BTC payment
/// - Type script: swap covenant (enforces atomic execution)
pub fn build_swap_transaction(
    offer: &Offer,
    acceptance: &SwapAcceptance,
    seller_rgbpp_cell: SwapCell,
    buyer_btc_cell: SwapCell,
    swap_covenant_type: CellType,
    current_block: u64,
) -> Result<SwapTransaction, String> {
    // Validate acceptance matches offer
    if acceptance.amount > offer.buy_amount {
        return Err("acceptance amount exceeds offer".to_string());
    }

    if acceptance.offer_id != offer_protocol::envelope(offer).offer_id {
        return Err("acceptance offer_id mismatch".to_string());
    }

    // Check expiry
    if current_block > offer.expiry {
        return Err("offer has expired".to_string());
    }

    // Build swap cell data
    let _swap_data = build_swap_cell_data(
        &offer.seller_lock_hash,
        &acceptance.buyer_lock_hash,
        offer.sell_amount,
        offer.buy_amount,
        offer.expiry,
    );

    // Output 0: buyer gets the RGB++ asset
    let buyer_output = SwapCell {
        cell_type: offer.sell_type.clone(),
        lock_hash: acceptance.buyer_lock_hash,
        capacity: seller_rgbpp_cell.capacity,
        data: vec![],
    };

    // Output 1: seller gets the BTC payment
    let seller_output = SwapCell {
        cell_type: offer.buy_type.clone(),
        lock_hash: offer.seller_lock_hash,
        capacity: buyer_btc_cell.capacity,
        data: vec![],
    };

    Ok(SwapTransaction {
        inputs: vec![seller_rgbpp_cell, buyer_btc_cell],
        outputs: vec![buyer_output, seller_output],
        covenant_type: swap_covenant_type,
        witnesses: vec![
            offer.signature.clone(),
            acceptance.signature.clone(),
        ],
        cell_deps: vec![offer.sell_type.clone(), offer.buy_type.clone()],
        header_deps: vec![],
    })
}

/// Serialize a swap transaction to JSON for Fiber network submission.
pub fn serialize_transaction(tx: &SwapTransaction) -> String {
    serde_json::to_string(tx).expect("serialization should not fail")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use offer_protocol::{create_offer_payload, build_offer, envelope};

    fn dummy_cell_type() -> CellType {
        CellType {
            code_hash: [0u8; 32],
            hash_type: 1,
            args: vec![0u8; 20],
        }
    }

    fn dummy_cell(cell_type: CellType, lock_hash: [u8; 32]) -> SwapCell {
        SwapCell {
            cell_type,
            lock_hash,
            capacity: 1_000_000_000, // 1 CKB
            data: vec![],
        }
    }

    #[test]
    fn test_build_swap_cell_data() {
        let seller = [1u8; 32];
        let buyer = [2u8; 32];
        let data = build_swap_cell_data(&seller, &buyer, 1000, 500, 100_000);

        assert_eq!(data.len(), 88);
        assert_eq!(&data[0..32], &seller);
        assert_eq!(&data[32..64], &buyer);
        assert_eq!(u64::from_le_bytes(data[64..72].try_into().unwrap()), 1000);
        assert_eq!(u64::from_le_bytes(data[72..80].try_into().unwrap()), 500);
        assert_eq!(u64::from_le_bytes(data[80..88].try_into().unwrap()), 100_000);
    }

    #[test]
    fn test_build_swap_transaction_success() {
        let sell_type = dummy_cell_type();
        let buy_type = dummy_cell_type();
        let seller_lock = [1u8; 32];
        let buyer_lock = [2u8; 32];

        let (payload, _) = create_offer_payload(
            sell_type.clone(),
            1000,
            buy_type.clone(),
            500,
            seller_lock,
            100_000,
        );
        let offer = build_offer(payload, vec![0xff; 64]);
        let env = envelope(&offer);

        let acceptance = SwapAcceptance {
            offer_id: env.offer_id,
            buyer_lock_hash: buyer_lock,
            amount: 500,
            signature: vec![0xee; 64],
        };

        let seller_cell = dummy_cell(sell_type, seller_lock);
        let buyer_cell = dummy_cell(buy_type, buyer_lock);
        let covenant_type = dummy_cell_type();

        let result = build_swap_transaction(
            &offer,
            &acceptance,
            seller_cell,
            buyer_cell,
            covenant_type,
            50_000, // before expiry
        );

        assert!(result.is_ok());
        let tx = result.unwrap();
        assert_eq!(tx.inputs.len(), 2);
        assert_eq!(tx.outputs.len(), 2);
    }

    #[test]
    fn test_build_swap_transaction_expired() {
        let sell_type = dummy_cell_type();
        let buy_type = dummy_cell_type();
        let seller_lock = [1u8; 32];
        let buyer_lock = [2u8; 32];

        let (payload, _) = create_offer_payload(
            sell_type.clone(),
            1000,
            buy_type.clone(),
            500,
            seller_lock,
            100, // expires at block 100
        );
        let offer = build_offer(payload, vec![0xff; 64]);
        let env = envelope(&offer);

        let acceptance = SwapAcceptance {
            offer_id: env.offer_id,
            buyer_lock_hash: buyer_lock,
            amount: 500,
            signature: vec![0xee; 64],
        };

        let seller_cell = dummy_cell(sell_type, seller_lock);
        let buyer_cell = dummy_cell(buy_type, buyer_lock);
        let covenant_type = dummy_cell_type();

        let result = build_swap_transaction(
            &offer,
            &acceptance,
            seller_cell,
            buyer_cell,
            covenant_type,
            200, // after expiry
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expired"));
    }

    #[test]
    fn test_build_swap_transaction_amount_mismatch() {
        let sell_type = dummy_cell_type();
        let buy_type = dummy_cell_type();
        let seller_lock = [1u8; 32];
        let buyer_lock = [2u8; 32];

        let (payload, _) = create_offer_payload(
            sell_type.clone(),
            1000,
            buy_type.clone(),
            500,
            seller_lock,
            100_000,
        );
        let offer = build_offer(payload, vec![0xff; 64]);
        let env = envelope(&offer);

        let acceptance = SwapAcceptance {
            offer_id: env.offer_id,
            buyer_lock_hash: buyer_lock,
            amount: 600, // more than offer allows
            signature: vec![0xee; 64],
        };

        let seller_cell = dummy_cell(sell_type, seller_lock);
        let buyer_cell = dummy_cell(buy_type, buyer_lock);
        let covenant_type = dummy_cell_type();

        let result = build_swap_transaction(
            &offer,
            &acceptance,
            seller_cell,
            buyer_cell,
            covenant_type,
            50_000,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds"));
    }
}
