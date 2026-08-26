//! Offer Protocol — Signed messages representing swap offers.
//!
//! An offer is a signed message from a seller:
//! "I will sell X units of RGB++ asset Y for Z units of BTC/asset W,
//!  valid until block N."

use blake2b_rs::Blake2bBuilder;
use k256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CellType {
    pub code_hash: [u8; 32],
    pub hash_type: u8,
    pub args: Vec<u8>,
}

impl CellType {
    pub fn hash(&self) -> [u8; 32] {
        let mut blake2b = Blake2bBuilder::new(32).build();
        blake2b.update(&self.code_hash);
        blake2b.update(&[self.hash_type]);
        blake2b.update(&self.args);
        let mut result = [0u8; 32];
        blake2b.finalize(&mut result);
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offer {
    pub sell_type: CellType,
    pub sell_amount: u64,
    pub buy_type: CellType,
    pub buy_amount: u64,
    pub seller_lock_hash: [u8; 32],
    pub expiry: u64,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferEnvelope {
    pub offer: Offer,
    pub offer_id: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferPayload {
    sell_type: CellType,
    sell_amount: u64,
    buy_type: CellType,
    buy_amount: u64,
    seller_lock_hash: [u8; 32],
    expiry: u64,
}

impl OfferPayload {
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialization should not fail")
    }
}

pub fn hash_payload(payload: &OfferPayload) -> [u8; 32] {
    let payload_bytes = payload.to_bytes();
    let mut blake2b = Blake2bBuilder::new(32).build();
    blake2b.update(&payload_bytes);
    let mut hash = [0u8; 32];
    blake2b.finalize(&mut hash);
    hash
}

pub fn create_offer_payload(
    sell_type: CellType,
    sell_amount: u64,
    buy_type: CellType,
    buy_amount: u64,
    seller_lock_hash: [u8; 32],
    expiry: u64,
) -> (OfferPayload, [u8; 32]) {
    let payload = OfferPayload {
        sell_type, sell_amount, buy_type, buy_amount, seller_lock_hash, expiry,
    };
    let hash = hash_payload(&payload);
    (payload, hash)
}

pub fn build_offer(payload: OfferPayload, signature: Vec<u8>) -> Offer {
    Offer {
        sell_type: payload.sell_type,
        sell_amount: payload.sell_amount,
        buy_type: payload.buy_type,
        buy_amount: payload.buy_amount,
        seller_lock_hash: payload.seller_lock_hash,
        expiry: payload.expiry,
        signature,
    }
}

pub fn envelope(offer: &Offer) -> OfferEnvelope {
    let payload = OfferPayload {
        sell_type: offer.sell_type.clone(),
        sell_amount: offer.sell_amount,
        buy_type: offer.buy_type.clone(),
        buy_amount: offer.buy_amount,
        seller_lock_hash: offer.seller_lock_hash,
        expiry: offer.expiry,
    };
    let offer_id = hash_payload(&payload);
    OfferEnvelope { offer: offer.clone(), offer_id }
}

/// Verify offer signature against a compressed secp256k1 public key (33 bytes).
pub fn verify_offer(offer: &Offer, seller_pubkey: &[u8]) -> bool {
    if offer.signature.is_empty() {
        return false;
    }

    let payload = OfferPayload {
        sell_type: offer.sell_type.clone(),
        sell_amount: offer.sell_amount,
        buy_type: offer.buy_type.clone(),
        buy_amount: offer.buy_amount,
        seller_lock_hash: offer.seller_lock_hash,
        expiry: offer.expiry,
    };

    let hash = hash_payload(&payload);
    let verifying_key = match VerifyingKey::from_sec1_bytes(seller_pubkey) {
        Ok(key) => key,
        Err(_) => return false,
    };

    let sig_len = offer.signature.len();
    let signature = if sig_len >= 64 {
        Signature::from_slice(&offer.signature[..64]).ok()
    } else {
        Signature::from_der(&offer.signature).ok()
    };

    match signature {
        Some(sig) => verifying_key.verify(&hash, &sig).is_ok(),
        None => false,
    }
}

pub fn is_expired(offer: &Offer, current_block: u64) -> bool {
    current_block > offer.expiry
}

pub fn serialize_offer(envelope: &OfferEnvelope) -> Vec<u8> {
    serde_json::to_vec(envelope).expect("serialization should not fail")
}

pub fn deserialize_offer(data: &[u8]) -> Result<OfferEnvelope, serde_json::Error> {
    serde_json::from_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use k256::ecdsa::signature::Signer;

    fn key_a() -> SigningKey {
        SigningKey::from_bytes(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ].into()).unwrap()
    }

    fn key_b() -> SigningKey {
        SigningKey::from_bytes(&[
            0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
            0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
            0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
            0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60,
        ].into()).unwrap()
    }

    fn pubkey_of(key: &SigningKey) -> Vec<u8> {
        key.verifying_key().to_sec1_bytes().to_vec()
    }

    fn lock_hash_of(pubkey: &[u8]) -> [u8; 32] {
        let mut blake2b = Blake2bBuilder::new(32).build();
        blake2b.update(pubkey);
        let mut hash = [0u8; 32];
        blake2b.finalize(&mut hash);
        hash
    }

    fn dummy_cell_type() -> CellType {
        CellType { code_hash: [0u8; 32], hash_type: 1, args: vec![0u8; 20] }
    }

    fn make_offer_with(key: &SigningKey, sell: u64, buy: u64, expiry: u64) -> Offer {
        let pk = pubkey_of(key);
        let lh = lock_hash_of(&pk);
        let (payload, hash) = create_offer_payload(
            dummy_cell_type(), sell, dummy_cell_type(), buy, lh, expiry,
        );
        let sig: k256::ecdsa::Signature = key.sign(&hash);
        let sig = sig.to_bytes().to_vec();
        build_offer(payload, sig)
    }

    #[test]
    fn test_create_and_envelope() {
        let offer = make_offer_with(&key_a(), 1000, 500, 100_000);
        let env = envelope(&offer);
        assert_eq!(env.offer.sell_amount, 1000);
        assert_eq!(env.offer.buy_amount, 500);
        assert_ne!(env.offer_id, [0u8; 32]);
    }

    #[test]
    fn test_verify_valid() {
        let offer = make_offer_with(&key_a(), 1000, 500, 100_000);
        let pk = pubkey_of(&key_a());
        assert!(verify_offer(&offer, &pk));
    }

    #[test]
    fn test_verify_wrong_key() {
        let offer = make_offer_with(&key_a(), 1000, 500, 100_000);
        let wrong_pk = pubkey_of(&key_b());
        assert!(!verify_offer(&offer, &wrong_pk));
    }

    #[test]
    fn test_verify_empty_signature() {
        let mut offer = make_offer_with(&key_a(), 1000, 500, 100_000);
        offer.signature = vec![];
        let pk = pubkey_of(&key_a());
        assert!(!verify_offer(&offer, &pk));
    }

    #[test]
    fn test_verify_tampered() {
        let mut offer = make_offer_with(&key_a(), 1000, 500, 100_000);
        offer.sell_amount = 9999;
        let pk = pubkey_of(&key_a());
        assert!(!verify_offer(&offer, &pk));
    }

    #[test]
    fn test_is_expired() {
        let offer = make_offer_with(&key_a(), 1000, 500, 100);
        assert!(!is_expired(&offer, 50));
        assert!(!is_expired(&offer, 100));
        assert!(is_expired(&offer, 101));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let offer = make_offer_with(&key_a(), 1000, 500, 100_000);
        let env = envelope(&offer);
        let serialized = serialize_offer(&env);
        let deserialized = deserialize_offer(&serialized).unwrap();
        assert_eq!(deserialized.offer.sell_amount, 1000);
        assert_eq!(deserialized.offer_id, env.offer_id);
    }

    #[test]
    fn test_verify_after_deserialization() {
        let offer = make_offer_with(&key_a(), 1000, 500, 100_000);
        let env = envelope(&offer);
        let pk = pubkey_of(&key_a());
        let serialized = serialize_offer(&env);
        let deserialized = deserialize_offer(&serialized).unwrap();
        assert!(verify_offer(&deserialized.offer, &pk));
    }
}
