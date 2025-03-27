from dataclasses import dataclass
from datetime import datetime
from typing import Optional

@dataclass
class NoncePermission:
    permission: str | dict[str, str]
    nonce: int

@dataclass
class PublicKey:
    access_key: NoncePermission
    public_key: str

@dataclass
class MpcKey:
    public_key: str
    account_id: str

@dataclass
class IntentActions:
    intent: str
    diff: dict[str, str]
    referral: Optional[str] = None
    receiver_id: Optional[str] = None
    tokens: Optional[dict[str, str]] = None
    memo: Optional[str] = None

@dataclass
class Intent:
    signer_id: str
    nonce: str
    verifying_contract: str
    deadline: datetime
    token_diffs: list[IntentActions]

@dataclass
class SignatureRequest:
    contract_id: str
    method_name: str
    args: str
    deposit: str
    nonce: str
    block_hash: str
    mpc_signer_pk: str
    account_pk_for_mpc: str
    gas: str = "50000000000000"
