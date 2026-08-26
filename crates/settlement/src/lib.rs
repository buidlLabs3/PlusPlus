//! Settlement Layer — Finalizes swaps on CKB after off-chain routing.
//!
//! Handles: settlement proof generation, on-chain anchoring,
//! dispute resolution, and force-close with latest signed state.

use k256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementProof {
    pub tx_hash: String,
    pub offer_id: String,
    pub seller_lock: String,
    pub buyer_lock: String,
    pub amount: u64,
    pub block_number: u64,
    pub timestamp: u64,
    /// 64-byte compact secp256k1 signature from seller
    pub seller_signature: Vec<u8>,
    /// 64-byte compact secp256k1 signature from buyer
    pub buyer_signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStateUpdate {
    pub channel_id: String,
    pub sequence: u64,
    pub balance_a: u64,
    pub balance_b: u64,
    /// 64-byte compact secp256k1 signature from party A
    pub signature_a: Vec<u8>,
    /// 64-byte compact secp256k1 signature from party B
    pub signature_b: Vec<u8>,
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SettlementStatus {
    Pending,
    Confirmed,
    Disputed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    pub dispute_id: String,
    pub initiator: String,
    pub reason: DisputeReason,
    pub status: SettlementStatus,
    pub evidence: Option<ChannelStateUpdate>,
    pub filed_block: u64,
    pub resolved_block: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DisputeReason {
    CounterpartyOffline,
    InvalidState,
    PaymentNotReceived,
    CapacityManipulation,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Hash settlement fields for signature verification.
fn settlement_hash(proof: &SettlementProof) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(proof.tx_hash.as_bytes());
    data.extend_from_slice(proof.offer_id.as_bytes());
    data.extend_from_slice(proof.seller_lock.as_bytes());
    data.extend_from_slice(proof.buyer_lock.as_bytes());
    data.extend_from_slice(&proof.amount.to_le_bytes());
    data.extend_from_slice(&proof.block_number.to_le_bytes());
    data.extend_from_slice(&proof.timestamp.to_le_bytes());

    let mut blake2b = blake2b_rs::Blake2bBuilder::new(32).build();
    blake2b.update(&data);
    let mut hash = [0u8; 32];
    blake2b.finalize(&mut hash);
    hash
}

/// Hash channel state update for signature verification.
fn state_hash(update: &ChannelStateUpdate) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(update.channel_id.as_bytes());
    data.extend_from_slice(&update.sequence.to_le_bytes());
    data.extend_from_slice(&update.balance_a.to_le_bytes());
    data.extend_from_slice(&update.balance_b.to_le_bytes());
    data.extend_from_slice(&update.block_number.to_le_bytes());

    let mut blake2b = blake2b_rs::Blake2bBuilder::new(32).build();
    blake2b.update(&data);
    let mut hash = [0u8; 32];
    blake2b.finalize(&mut hash);
    hash
}

/// Verify a 64-byte compact secp256k1 signature against a hash and pubkey.
fn verify_signature(hash: &[u8; 32], sig_bytes: &[u8], pubkey_bytes: &[u8]) -> bool {
    if sig_bytes.len() < 64 || pubkey_bytes.is_empty() {
        return false;
    }
    let verifying_key = match VerifyingKey::from_sec1_bytes(pubkey_bytes) {
        Ok(k) => k,
        Err(_) => return false,
    };
    let signature = match Signature::from_slice(&sig_bytes[..64]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    verifying_key.verify(hash, &signature).is_ok()
}

// ---------------------------------------------------------------------------
// Settlement operations
// ---------------------------------------------------------------------------

pub fn generate_settlement_proof(
    tx_hash: &str,
    offer_id: &str,
    seller_lock: &str,
    buyer_lock: &str,
    amount: u64,
    block_number: u64,
    timestamp: u64,
    seller_signature: Vec<u8>,
    buyer_signature: Vec<u8>,
) -> SettlementProof {
    SettlementProof {
        tx_hash: tx_hash.to_string(),
        offer_id: offer_id.to_string(),
        seller_lock: seller_lock.to_string(),
        buyer_lock: buyer_lock.to_string(),
        amount,
        block_number,
        timestamp,
        seller_signature,
        buyer_signature,
    }
}

/// Verify settlement proof with cryptographic signature checks.
/// Requires the public keys (33-byte compressed secp256k1) for both parties.
pub fn verify_settlement_proof(
    proof: &SettlementProof,
    seller_pubkey: &[u8],
    buyer_pubkey: &[u8],
) -> bool {
    if proof.amount == 0 {
        return false;
    }
    if proof.seller_lock.len() != 64 || proof.buyer_lock.len() != 64 {
        return false;
    }

    let hash = settlement_hash(proof);

    verify_signature(&hash, &proof.seller_signature, seller_pubkey)
        && verify_signature(&hash, &proof.buyer_signature, buyer_pubkey)
}

pub fn create_state_update(
    channel_id: &str,
    sequence: u64,
    balance_a: u64,
    balance_b: u64,
    signature_a: Vec<u8>,
    signature_b: Vec<u8>,
    block_number: u64,
) -> ChannelStateUpdate {
    ChannelStateUpdate {
        channel_id: channel_id.to_string(),
        sequence,
        balance_a,
        balance_b,
        signature_a,
        signature_b,
        block_number,
    }
}

/// Verify channel state update with cryptographic signature checks.
pub fn verify_state_update(
    update: &ChannelStateUpdate,
    party_a_pubkey: &[u8],
    party_b_pubkey: &[u8],
) -> bool {
    let hash = state_hash(update);

    verify_signature(&hash, &update.signature_a, party_a_pubkey)
        && verify_signature(&hash, &update.signature_b, party_b_pubkey)
}

pub fn is_newer_state(a: &ChannelStateUpdate, b: &ChannelStateUpdate) -> bool {
    a.sequence > b.sequence
}

pub fn file_dispute(
    dispute_id: &str,
    initiator: &str,
    reason: DisputeReason,
    evidence: Option<ChannelStateUpdate>,
    filed_block: u64,
) -> Dispute {
    Dispute {
        dispute_id: dispute_id.to_string(),
        initiator: initiator.to_string(),
        reason,
        status: SettlementStatus::Disputed,
        evidence,
        filed_block,
        resolved_block: 0,
    }
}

pub fn resolve_dispute(
    dispute: &mut Dispute,
    latest_state: &ChannelStateUpdate,
    resolved_block: u64,
    party_a_pubkey: &[u8],
    party_b_pubkey: &[u8],
) -> Result<(), String> {
    if dispute.status != SettlementStatus::Disputed {
        return Err("dispute not in disputed state".to_string());
    }

    if !verify_state_update(latest_state, party_a_pubkey, party_b_pubkey) {
        return Err("invalid state update signatures".to_string());
    }

    if let Some(ref evidence) = dispute.evidence {
        if !is_newer_state(latest_state, evidence) {
            return Err("new state is not newer than evidence".to_string());
        }
    }

    dispute.status = SettlementStatus::Confirmed;
    dispute.resolved_block = resolved_block;
    dispute.evidence = Some(latest_state.clone());

    Ok(())
}

pub fn force_close(
    latest_state: &ChannelStateUpdate,
    current_block: u64,
) -> Result<ForceCloseResult, String> {
    if latest_state.signature_a.is_empty() || latest_state.signature_b.is_empty() {
        return Err("missing signatures".to_string());
    }

    if current_block > latest_state.block_number + 1000 {
        return Err("state too old for force-close".to_string());
    }

    Ok(ForceCloseResult {
        channel_id: latest_state.channel_id.clone(),
        final_balance_a: latest_state.balance_a,
        final_balance_b: latest_state.balance_b,
        sequence: latest_state.sequence,
        closing_block: current_block,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceCloseResult {
    pub channel_id: String,
    pub final_balance_a: u64,
    pub final_balance_b: u64,
    pub sequence: u64,
    pub closing_block: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use k256::ecdsa::signature::Signer;

    fn key_a() -> SigningKey {
        SigningKey::from_bytes(&[0x01; 32].into()).unwrap()
    }

    fn key_b() -> SigningKey {
        SigningKey::from_bytes(&[0x02; 32].into()).unwrap()
    }

    fn pubkey_hex(key: &SigningKey) -> String {
        hex::encode(key.verifying_key().to_sec1_bytes())
    }

    /// Compute a 64-char lock hash from a signing key (blake2b-256 of the pubkey).
    fn lock_hash_hex(key: &SigningKey) -> String {
        use blake2b_rs::Blake2bBuilder;
        let pubkey = key.verifying_key().to_sec1_bytes();
        let mut blake2b = Blake2bBuilder::new(32).build();
        blake2b.update(&pubkey);
        let mut hash = [0u8; 32];
        blake2b.finalize(&mut hash);
        hex::encode(hash)
    }

    fn sign_hash(key: &SigningKey, hash: &[u8; 32]) -> Vec<u8> {
        let sig: k256::ecdsa::Signature = key.sign(hash);
        sig.to_bytes().to_vec()
    }

    #[test]
    fn test_settlement_proof_valid_signatures() {
        let proof = generate_settlement_proof(
            "tx_hash_123", "offer_456",
            &lock_hash_hex(&key_a()), &lock_hash_hex(&key_b()),
            500, 1000, 1700000000,
            vec![], vec![],
        );
        let hash = settlement_hash(&proof);
        let mut proof = proof;
        proof.seller_signature = sign_hash(&key_a(), &hash);
        proof.buyer_signature = sign_hash(&key_b(), &hash);

        assert!(verify_settlement_proof(
            &proof,
            &key_a().verifying_key().to_sec1_bytes(),
            &key_b().verifying_key().to_sec1_bytes(),
        ));
    }

    #[test]
    fn test_settlement_proof_wrong_seller_key() {
        let proof = generate_settlement_proof(
            "tx", "offer",
            &lock_hash_hex(&key_a()), &lock_hash_hex(&key_b()),
            500, 1000, 1700000000,
            vec![], vec![],
        );
        let hash = settlement_hash(&proof);
        let mut proof = proof;
        proof.seller_signature = sign_hash(&key_b(), &hash); // wrong key
        proof.buyer_signature = sign_hash(&key_b(), &hash);

        assert!(!verify_settlement_proof(
            &proof,
            &key_a().verifying_key().to_sec1_bytes(),
            &key_b().verifying_key().to_sec1_bytes(),
        ));
    }

    #[test]
    fn test_state_update_valid() {
        let mut update = create_state_update("ch1", 5, 300, 700, vec![], vec![], 1000);
        let hash = state_hash(&update);
        update.signature_a = sign_hash(&key_a(), &hash);
        update.signature_b = sign_hash(&key_b(), &hash);

        assert!(verify_state_update(
            &update,
            &key_a().verifying_key().to_sec1_bytes(),
            &key_b().verifying_key().to_sec1_bytes(),
        ));
    }

    #[test]
    fn test_state_update_empty_signatures() {
        let update = create_state_update("ch1", 5, 300, 700, vec![], vec![], 1000);
        assert!(!verify_state_update(
            &update,
            &key_a().verifying_key().to_sec1_bytes(),
            &key_b().verifying_key().to_sec1_bytes(),
        ));
    }

    #[test]
    fn test_is_newer_state() {
        let a = create_state_update("ch", 10, 500, 500, vec![], vec![], 1000);
        let b = create_state_update("ch", 5, 300, 700, vec![], vec![], 900);
        assert!(is_newer_state(&a, &b));
        assert!(!is_newer_state(&b, &a));
    }

    #[test]
    fn test_force_close() {
        let state = create_state_update("ch", 10, 400, 600, vec![0xff; 64], vec![0xff; 64], 1000);
        let result = force_close(&state, 1500);
        assert!(result.is_ok());
    }

    #[test]
    fn test_force_close_too_old() {
        let state = create_state_update("ch", 10, 400, 600, vec![0xff; 64], vec![0xff; 64], 100);
        let result = force_close(&state, 2000);
        assert!(result.is_err());
    }
}
