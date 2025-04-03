use near_sdk::PublicKey;
use omni_transaction::near::types::PublicKey as OmniPublicKey;
use omni_transaction::near::types::Secp256K1PublicKey;
use sha2::{Digest, Sha256};

pub fn hash_payload(payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.finalize().into()
}

pub fn convert_pk_to_omni(pk: &PublicKey) -> omni_transaction::near::types::PublicKey {
    // TODO We might need to expand this to support ETH/other curve types
    let public_key_data = &pk.as_bytes()[1..]; // Skipping the first byte which is the curve type

    //const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
    const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;
    let ed25519_key: [u8; SECP256K1_PUBLIC_KEY_LENGTH] = public_key_data
        .try_into()
        .expect("Failed to convert public key");

    OmniPublicKey::SECP256K1(Secp256K1PublicKey::from(ed25519_key))
}
