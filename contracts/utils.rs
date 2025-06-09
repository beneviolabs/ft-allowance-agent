use std::str::FromStr;

use near_sdk::PublicKey;
use omni_transaction::near::types::PublicKey as OmniPublicKey;
use omni_transaction::near::types::{ED25519PublicKey, Secp256K1PublicKey};
use sha2::{Digest, Sha256};

pub fn hash_payload(payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.finalize().into()
}

pub fn convert_pk_to_omni(pk: &String) -> omni_transaction::near::types::PublicKey {
    let (curve_type, pub_key) = if pk.starts_with("ed25519:") {
        (
            near_sdk::CurveType::ED25519,
            &PublicKey::from_str(pk).unwrap(),
        )
    } else if pk.starts_with("secp256k1:") {
        (
            near_sdk::CurveType::SECP256K1,
            &PublicKey::from_str(pk).unwrap(),
        )
    } else {
        panic!("Invalid public key format: {}", pk)
    };

    let public_key_data = &pub_key.as_bytes()[1..]; // Skipping the first byte which is the curve type

    match curve_type {
        near_sdk::CurveType::ED25519 => {
            const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
            let ed25519_key: [u8; ED25519_PUBLIC_KEY_LENGTH] =
                (*public_key_data).try_into().unwrap_or_else(|_| {
                    panic!(
                        "Failed to convert ED25519 key, expected length {}, got {}",
                        ED25519_PUBLIC_KEY_LENGTH,
                        public_key_data.len()
                    )
                });
            OmniPublicKey::ED25519(ED25519PublicKey::from(ed25519_key))
        }
        near_sdk::CurveType::SECP256K1 => {
            const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;
            let secp_key: [u8; SECP256K1_PUBLIC_KEY_LENGTH] =
                (*public_key_data).try_into().unwrap_or_else(|_| {
                    panic!(
                        "Failed to convert SECP256K1 key, expected length {}, got {}",
                        SECP256K1_PUBLIC_KEY_LENGTH,
                        public_key_data.len()
                    )
                });
            OmniPublicKey::SECP256K1(Secp256K1PublicKey::from(secp_key))
        }
    }
}
