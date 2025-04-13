from dataclasses import dataclass, asdict
from datetime import datetime
from typing import Optional, Union, Dict


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
    permission: Union[str, Dict[str, str]]
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
class MultiActionSignatureRequest:
    contract_id: str
    actions_json: str
    nonce: str
    block_hash: str
    mpc_signer_pk: str
    account_pk_for_mpc: str


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
class SignMessageSignatureRequest(DictMixin):
    contract_id: str
    args: str
    deposit: str
    nonce: str
    block_hash: str
    mpc_signer_pk: str
    account_pk_for_mpc: str
    method_name: Optional[str] = None
    gas: str = "50000000000000"

@dataclass
class QuoteRequest:
    dry: bool
    swap_type: str
    slippage_tolerance: int
    origin_asset: str
    deposit_type: str
    destination_asset: str
    amount: str
    refund_to: str
    refund_type: str
    recipient: str
    recipient_type: str
    deadline: str
    referral: str
    quote_waiting_time_ms: int


@dataclass
class Quote(DictMixin):
    deposit_address: str
    amount_in: str
    amount_in_formatted: str
    amount_in_usd: str
    min_amount_in: str
    amount_out: str
    amount_out_formatted: str
    amount_out_usd: str
    min_amount_out: str
    deadline: str
    time_when_inactive: str
    time_estimate: int


@dataclass
class OneClickQuote(DictMixin):
    timestamp: str
    signature: str
    quoteRequest: QuoteRequest
    quote: Quote
