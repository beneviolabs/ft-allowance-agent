use near_sdk::PublicKey;
use omni_transaction::near::types::ED25519PublicKey;
use omni_transaction::near::types::PublicKey as OmniPublicKey;
use sha2::{Digest, Sha256};

pub fn hash_payload(payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.finalize().into()
}

pub fn convert_pk_to_omni(pk: &String) -> omni_transaction::near::types::PublicKey {
    let trimmed_pk = pk.strip_prefix("ed25519:").unwrap_or(pk);
    let pk_parts = bs58::decode(trimmed_pk).into_vec().unwrap();

    let pub_key: near_sdk::PublicKey =
        PublicKey::from_parts(near_sdk::CurveType::ED25519, pk_parts).unwrap();

    // TODO We might need to expand this to support ETH/other curve types
    let public_key_data = &pub_key.as_bytes()[1..]; // Skipping the first byte which is the curve type

    const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
    //const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;
    let ed25519_key: [u8; ED25519_PUBLIC_KEY_LENGTH] =
        public_key_data.try_into().unwrap_or_else(|_| {
            panic!(
                "Failed to convert public key, expected length {}, got {}",
                ED25519_PUBLIC_KEY_LENGTH,
                public_key_data.len()
            )
        });

    OmniPublicKey::ED25519(ED25519PublicKey::from(ed25519_key))
}
