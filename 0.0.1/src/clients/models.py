from dataclasses import asdict, dataclass
from datetime import datetime
from typing import Optional


@dataclass
class DictMixin:
    """Base class providing dictionary conversion functionality"""

    def dict(self) -> dict:
        """
        Convert dataclass to dictionary, excluding None values.

        Returns:
            dict: Dictionary representation of the dataclass
        """
        return {k: v for k, v in asdict(self).items() if v is not None}


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


@dataclass
class Intent(DictMixin):
    signer_id: str
    nonce: str
    verifying_contract: str
    deadline: datetime
    intents: list[IntentActions]


@dataclass
class SignatureRequest(DictMixin):
    contract_id: str
    args: str
    deposit: str
    nonce: str
    block_hash: str
    mpc_signer_pk: str
    account_pk_for_mpc: str
    method_name: Optional[str] = None
    gas: str = "50000000000000"
