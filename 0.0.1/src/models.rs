use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    AccountId,
};
use omni_transaction::near::types::{Action, BlockHash, PublicKey, Signature, U64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BigR {
    pub affine_point: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScalarValue {
    pub scalar: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignatureResponse {
    pub big_r: BigR,
    pub s: ScalarValue,
    pub recovery_id: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct SignRequest {
    pub payload: Vec<u8>,
    pub path: String,
    pub key_version: u32,
}

// port of the private struct from omni-transaction-rs https://github.com/near/omni-transaction-rs/blob/fefa9f2987c7112a546ca7308d7f064e9fed267f/src/near/near_transaction.rs#L54
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct NearTransaction {
    /// An account on which behalf transaction is signed
    pub signer_id: AccountId,
    /// A public key of the access key which was used to sign an account.
    /// Access key holds permissions for calling certain kinds of actions.
    pub signer_public_key: PublicKey,
    /// Nonce is used to determine order of transaction in the pool.
    /// It increments for a combination of `signer_id` and `public_key`
    pub nonce: U64,
    /// Receiver account for this transaction
    pub receiver_id: AccountId,
    /// The hash of the block in the blockchain on top of which the given transaction is valid
    pub block_hash: BlockHash,
    /// A list of actions to be applied
    pub actions: Vec<Action>,
}

/// Signed NEAR transaction abstraction
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SignedTransaction {
    pub transaction: NearTransaction,
    pub signature: Signature,
}

impl NearTransaction {
    pub fn build_for_signing(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("failed to serialize NEAR transaction")
    }

    pub fn build_with_signature(&self, signature: Signature) -> Vec<u8> {
        let signed_tx = SignedTransaction {
            transaction: self.clone(),
            signature,
        };
        borsh::to_vec(&signed_tx).expect("failed to serialize NEAR transaction")
    }

    pub fn from_json(json: &str) -> Result<Self, near_sdk::serde_json::Error> {
        near_sdk::serde_json::from_str(json)
    }
}
