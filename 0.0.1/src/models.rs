use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
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
