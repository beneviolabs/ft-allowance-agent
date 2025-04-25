import base64
import json
import logging
import os
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from typing import Dict, List, Optional, TypeVar

import aiohttp
import requests
from dotenv import load_dotenv

from src.models import (
    Intent,
    IntentActions,
    MpcKey,
    MultiActionSignatureRequest,
    OneClickQuote,
    PublicKey,
    SignMessageSignatureRequest,
)
from src.utils import get_usdc_token_out_type, get_usdt_token_out_type, usd_to_base6

# Expose this logger config when testing these methods directly, without using the AI interface
# from .logger_config import configure_logging
# configure_logging()

logger = logging.getLogger(__name__)


class OneClickClient:
    BASE_URL = "https://1click.chaindefuser.com/v0"

    def __init__(self, env=None):
        self.session = None
        self.env = env
        self.env.add_system_log("OneClickClient initialized", logging.DEBUG)

    async def _ensure_session(self):
        """Ensures a client session exists"""
        if self.session is None:
            self.session = aiohttp.ClientSession()
        return self.session

    async def close(self):
        await self.session.close()

    async def get_supported_tokens(self) -> List[Dict]:
        """Fetch list of supported tokens from 1click API"""
        try:
            session = await self._ensure_session()
            async with session.get(f"{self.BASE_URL}/tokens") as response:
                if response.status == 200:
                    data = await response.json()
                    self.env.add_system_log(f"Got supported tokens: {len(data)} tokens", logging.DEBUG)
                    return data
                else:
                    return []
        except Exception as e:
            self.env.add_system_log(f"Failed to get tokens: {e}", logging.ERROR)
            raise

    def get_quote(
        self,
        token_in: str,
        token_out: str,
        amount_in: Decimal,
        depositor_address: str,
        recipient: str,
        dry: bool = True,
        slippage_tolerance: int = 100,
        deadline: Optional[str] = None,
    ) -> Optional[OneClickQuote]:
        """
        Get quote for swapping tokens using 1click API

        Args:
            token_in: Input token identifier (e.g. "nep141:wrap.near")
            token_out: Output token identifier (e.g. "nep141:usdc.near")
            amount_in: Amount of input token
            dry: If True, returns quote without executing swap
            slippage_tolerance: Maximum allowed slippage (default 1%)
            refund_to: Address to refund to if swap fails
            recipient: Address to receive swapped tokens
            deadline: Timestamp when a refund will be triggered
        """
        self.env.add_system_log(f"Getting quote for {amount_in} {token_in} to {token_out}", logging.DEBUG)

        try:
            # Set deadline to 15 minutes from now if not provided
            if deadline is None:
                future_time = datetime.now(timezone.utc) + timedelta(minutes=15)
                deadline = future_time.strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"
                self.env.add_system_log(f"Generated deadline: {deadline}", logging.DEBUG)

            payload = {
                "dry": dry,
                # request a quote given an exact input amount. The refundTo address will always receive excess tokens back even after the swap is complete.
                "swapType": "EXACT_INPUT",
                "slippageTolerance": slippage_tolerance,
                "originAsset": token_in,
                # For deposits orignating from a near account.  Otherwise use "INTENTS"
                "depositType": "INTENTS",
                "destinationAsset": token_out,
                # denoted in the smallest unit of the specified currency (e.g., wei for ETH).
                "amount": str(amount_in),
                "refundTo": depositor_address,
                # or use "ORIGIN_CHAIN" to refund the assets to account on their original chain
                "refundType": "INTENTS",
                "recipient": recipient,  # The format should match recipientType
                "recipientType": "DESTINATION_CHAIN",
                "deadline": deadline,
                "referral": "benevio-labs.near",
            }
            self.env.add_system_log(f"Sending quote request with payload: {payload}", logging.DEBUG)

            response = requests.post(f"{self.BASE_URL}/quote", json=payload)

            if response.status_code in [200, 201]:
                data = response.json()
                if not dry:
                    self.env.add_system_log(
                        f"Swap initiated with deposit address: {data.get('deposit_address')}", logging.INFO
                    )
                return OneClickQuote(**data)
            else:
                self.env.add_system_log(f"Error response: {response.text}", logging.ERROR)
                return None

        except Exception as e:
            self.env.add_system_log(f"Error getting quote: {e}", logging.ERROR)
            raise

    def check_transaction_status(self, deposit_address: str) -> Dict:
        """Check status of a transaction using deposit address"""
        try:
            response = requests.get(
            f"{self.BASE_URL}/status?depositAddress={deposit_address}"
            )
            response.raise_for_status()

            if response.status_code == 200:
                data = response.json()
                self.env.add_system_log(f"Got transaction status: {data}", logging.DEBUG)
                return data
            else:
                error_body = response.text
                self.env.add_system_log(f"Failed to get status: {error_body}", logging.ERROR)
                return {}

        except requests.exceptions.RequestException as e:
            self.env.add_system_log(f"Request failed: {str(e)}", logging.ERROR)
            raise
        except Exception as e:
            self.env.add_system_log(f"Error checking transaction status: {str(e)}", logging.ERROR)
            raise


class NearMpcClient:
    def __init__(self, network: str = "testnet", env=None):
        self.env = env
        # Load environment variables
        load_dotenv()

        # Validate required environment variables
        self._validate_env_vars()

        self.oneclickapi = OneClickClient(env=env)
        self.network = network
        self.rpc_url = f"https://rpc.{network}.fastnear.com"
        self.mpc_signer = (
            "v1.signer-prod.testnet" if network == "testnet" else "v1.signer"
        )
        self._derived_key: Optional[str] = None
        self._account_public_key: Optional[PublicKey] = None

    def close(self):
        self.oneclickapi.session.close()


    def get_stablecoin_quotes(
        self,
        token_in: str,
        amount_in: str,
        requested_by_address: str,
        dry: bool = True,
    ) -> Dict[str, OneClickQuote]:
        """Get quotes for both USDC and USDT swaps"""
        quotes = {}

        if not requested_by_address.startswith('agent.'):
            self.env.add_system_log("Expected a proxy account address - must start with 'agent.'", logging.ERROR)
            raise ValueError("requested_by_address must start with 'agent.'")

        # get stablecoin identifiers depending on the input token
        stablecoins = {
            "USDC": get_usdc_token_out_type(token_in),
            "USDT": get_usdt_token_out_type(token_in),
        }

        self.env.add_system_log(f"fetching quotes for stablecoins: {stablecoins}", logging.DEBUG)

        for name, token_id in stablecoins.items():
            quote = self.oneclickapi.get_quote(
                token_in,
                token_id,
                amount_in,
                requested_by_address,
                requested_by_address,
                dry,
            )
            if quote:
                quotes[name] = quote

        self.env.add_system_log(f"Got quotes: {json.dumps({k: v.dict() for k,v in quotes.items()})}", logging.INFO)
        return quotes


    QuoteType = TypeVar('QuoteType')

    def select_best_stablecoin_quote(
        self,
        quotes: Dict[str, QuoteType]
    ) -> Optional[QuoteType]:
        """
        Select the best quote between USDC and USDT based on minimum amount out with slippage taken into account.

        Args:
            quotes: Dictionary with 'USDC' and/or 'USDT' keys containing quote objects
            logger: Optional logging function for debug messages

        Returns:
            Optional[QuoteType]: Best quote object or None if no valid quotes
        """
        for token, quote_info in quotes.items():
            self.env.add_system_log(f"Quote for {token}: {quote_info.quote}", logging.DEBUG)

        best_quote = None
        if quotes.get("USDC") and quotes.get("USDT"):
            usdc_amount = Decimal(quotes["USDC"].quote.get("minAmountOut", 0))
            usdt_amount = Decimal(quotes["USDT"].quote.get("minAmountOut", 0))
            best_quote = quotes["USDC"] if usdc_amount > usdt_amount else quotes["USDT"]
        elif quotes.get("USDC"):
            best_quote = quotes["USDC"]
        elif quotes.get("USDT"):
            best_quote = quotes["USDT"]

        return best_quote

    def _validate_env_vars(self):
        """Validate that all required environment variables are set"""
        required_vars = []

        missing_vars = [var for var in required_vars if not os.getenv(var)]

        if missing_vars:
            error_msg = (
                f"Missing required environment variables: {', '.join(missing_vars)}"
            )
            self.env.add_system_log(error_msg, logging.ERROR)
            raise ValueError(error_msg)

    def _fetch_latest_block_hash(self) -> str:
        """Fetches the latest block hash from the NEAR network"""
        try:
            response = requests.post(
                self.rpc_url,
                json={
                    "jsonrpc": "2.0",
                    "id": "benevio.dev",
                    "method": "block",
                    "params": {"finality": "final"},
                },
            )
            response.raise_for_status()
            data = response.json()
            block_hash = data["result"]["header"]["hash"]
            self.env.add_system_log(f"Got block hash: {block_hash}", logging.INFO)
            return block_hash

        except requests.exceptions.RequestException as e:
            self.env.add_system_log(f"RPC request failed: {str(e)}", logging.ERROR)
            raise
        except Exception as e:
            self.env.add_system_log(f"Failed to fetch block hash: {str(e)}", logging.ERROR)
            raise


    def derive_mpc_key(self, proxy_account_id: str) -> MpcKey:
        """Derives MPC key for given account"""
        self.env.add_system_log(f"Deriving MPC key for account: {proxy_account_id}", logging.DEBUG)
        try:
            keys = self._get_public_key(proxy_account_id)
            pk = self._get_full_access_key(keys)
            if not pk:
                raise ValueError("No full access key found")
            args = {
                "predecessor": proxy_account_id,
                "path": pk,
            }
            response = self._query_rpc(
                "call_function",
                {
                    "method_name": "derived_public_key",
                    "account_id": self.mpc_signer,
                    "args_base64": base64.b64encode(json.dumps(args).encode()).decode(),
                },
            )
            self._derived_key = self._parse_view_result(response)
            self.env.add_system_log(f"Successfully derived MPC key {self._derived_key}", logging.INFO)
            return MpcKey(
                public_key=self._derived_key,
                account_id=proxy_account_id,
            )
        except Exception as e:
            self.env.add_system_log(f"Failed to derive MPC key: {str(e)}", logging.ERROR)
            raise

    def _get_full_access_key(self, keys: list[PublicKey]) -> Optional[PublicKey]:
        """Finds the first full access ED25519 public key from a list of keys."""
        try:
            for key in keys:
                permission = key["access_key"]["permission"]
                public_key = key["public_key"]

                if permission == "FullAccess" and public_key.startswith("ed25519:"):
                    self.env.add_system_log(f"Found full access key: {public_key}", logging.INFO)
                    self._account_public_key = key["public_key"]
                    return self._account_public_key

            self.env.add_system_log("No full access ED25519 key found", logging.ERROR)
            return None

        except Exception as e:
            self.env.add_system_log(f"Error parsing keys: {str(e)}", logging.ERROR)
            raise

    def _get_public_key(self, account_id: str) -> list[PublicKey]:
        """Returns public keys for a given account"""
        try:
            response = self._query_rpc(
                "view_access_key_list", {"finality": "final", "account_id": account_id}
            )
            if "keys" in response and len(response["keys"]) > 0:
                return response["keys"]
            else:
                raise ValueError(f"No keys found for account {account_id}")
        except Exception as e:
            self.env.add_system_log(f"Failed to get public key: {str(e)}", logging.ERROR)
            raise

    def _get_next_nonce(self, proxy_account_id: str) -> int:
        """Calculate a next nonce for a given account"""

        if self._derived_key is None:
            self.derive_mpc_key(proxy_account_id)
        try:
            self.env.add_system_log(f"Using derived key: {self._derived_key}", logging.INFO)
            response = self._query_rpc(
                "view_access_key",
                {
                    "finality": "final",
                    "account_id": proxy_account_id,
                    "public_key": self._derived_key,
                },
            )
            if "nonce" in response:
                return str(response["nonce"] + 10)
            else:
                raise ValueError(f"No nonce found for account {proxy_account_id}")
        except Exception as e:
            self.env.add_system_log(f"Failed to get next nonce: {str(e)}", logging.ERROR)
            raise

    def _request_multi_action_signature(
        self,
        contract_id: str,
        actions_json: str,
        proxy_account_id: str,
    ) -> Dict:
        """
        Request signature for multiple actions using MPC

        Args:
            contract_id: Target contract for the actions
            actions_json: JSON string of actions to be signed
            proxy_account_id: Account ID of the proxy contract

        Returns:
            Dict: Response from the proxy contract
        """
        try:
            # Get latest block hash if not provided
            block_hash = self._fetch_latest_block_hash()

            # Create signature request
            signature_request = MultiActionSignatureRequest(
                contract_id=contract_id,
                actions_json=actions_json,
                nonce=self._get_next_nonce(proxy_account_id),
                block_hash=block_hash,
                mpc_signer_pk=self._derived_key,
                account_pk_for_mpc=self._account_public_key,
            )

            self.env.add_system_log(
                f"Requesting multi-action signature  {signature_request.dict()}", logging.DEBUG
            )

            # Call proxy contract
            response = self._call_contract(
                proxy_account_id, "request_signature", signature_request.dict()
            )

            return response

        except Exception as e:
            self.env.add_system_log(f"Failed to request multi-action signature: {str(e)}", logging.ERROR)
            raise

    def _request_intent_signature(
        self, proxy_account_id: str, intent: Intent, block_hash: str
    ) -> str:
        """Publishes swap intent to Defuse network"""
        self.env.add_system_log(f"Publishing swap intent: {intent}", logging.DEBUG)
        try:
            TGAS = 1_000_000_000_000
            DEFAULT_ATTACHED_GAS = 100 * TGAS

            # Create signature request with cleaned intent
            signature_request = SignMessageSignatureRequest(
                contract_id=intent.verifying_contract,
                args=json.dumps(intent.dict()),
                deposit=str(DEFAULT_ATTACHED_GAS),
                nonce=self._get_next_nonce(proxy_account_id),
                block_hash=block_hash,
                mpc_signer_pk=self._derived_key,
                account_pk_for_mpc=self._account_public_key,
            )

            self.env.add_system_log(f"Signature request details: {signature_request.dict()}", logging.DEBUG)

            # Request signature
            result = self._call_contract(
                proxy_account_id, "request_sign_message", signature_request.dict()
            )
            success_value = result.status.get("SuccessValue")
            self.env.add_system_log(f"Successfully requested signature: {success_value}", logging.INFO)
            return self._decode_success_value(success_value)
        except Exception as e:
            self.env.add_system_log(f"Signature request failed: {str(e)}", logging.ERROR)
            raise

    async def sign_intent(
        self,
        proxy_account_id: str,
        token_in_address: str,
        token_out_address: str,
        token_in_amount: str,
        token_out_amount: str,
        quote_hash: str,
        deadline: str,
        nonce: str,
    ) -> dict:
        """Creates intent object with proper formatting"""
        self.env.add_system_log(f"Creating intent for signer: {proxy_account_id}", logging.DEBUG)
        try:
            if self.network != "mainnet":
                self.env.add_system_log("Intent creation attempted on non-mainnet network", logging.ERROR)
                raise ValueError("Intent creation is only supported on mainnet")

            token_diffs = [
                IntentActions(
                    intent="token_diff",
                    diff={
                        token_in_address: "-" + token_in_amount,
                        token_out_address: token_out_amount,
                    },
                )
            ]

            intent = Intent(
                signer_id=proxy_account_id,
                nonce=nonce,
                verifying_contract="intents.near",
                deadline=deadline,
                intents=token_diffs,
            )
            self.env.add_system_log(f"Successfully created intent  {intent}", logging.INFO)

            block_hash = self._fetch_latest_block_hash()

            signature = await self._request_intent_signature(
                proxy_account_id, intent, block_hash
            )
            signature = "ed25519:" + signature

            result = {
                "signature": signature,
                "intent": intent.dict(),
                "quote_hash": quote_hash,
                "public_key": self._derived_key,
            }
            self.env.add_system_log(f"Returning result: {result}", logging.DEBUG)
            return result

        except Exception as e:
            self.env.add_system_log(f"Intent creation failed: {str(e)}", logging.ERROR)
            raise

    def _call_contract(
        self, proxy_account_id: str, method_name: str, params: dict
    ) -> dict:
        """Signed as the agentic account, this function sends a transaction for an MPC signature request
to the user's proxy account."""
        try:
            worker_url = os.environ.get("near_call_worker")

            self.env.add_system_log(
                f"Calling near contracts via: {worker_url}",
                logging.DEBUG
            )

            response = requests.post(
                worker_url,
                json={
                    "proxy_account_id": proxy_account_id,
                    "method_name": method_name,
                    "params": params,
                }
            )

            if response.status_code != 200:
                error_text = response.text
                self.env.add_system_log(
                    f"Near call failed: {error_text}",
                    logging.ERROR
                )
                raise Exception(f"Near call failed: {error_text}")

            result = response.json()

            if "SuccessValue" not in result["status"]:
                raise Exception(
                    f"Contract call failed with status: {result['status']}"
                )
            return result

        except Exception as e:
            self.env.add_system_log(
                f"Contract call failed: {str(e)}",
                logging.ERROR
            )
            raise

    def _decode_success_value(self, encoded_value: str) -> str:
        """
        Decodes a Base64 encoded success value that contains a Base58 string

        Args:
            encoded_value (str): Base64 encoded string containing Base58 data

        Returns:
            str: Decoded Base58 string
        """
        # First decode from Base64
        base64_decoded = base64.b64decode(encoded_value)
        self.env.add_system_log(f"Decoding Base64 value: {base64_decoded}", logging.DEBUG)

        # Remove quotes if present and decode from Base58
        clean_value = base64_decoded.decode("utf-8").strip('"')
        self.env.add_system_log(f"Decoding Base64 value: {clean_value}", logging.DEBUG)
        return clean_value

    def _query_rpc(self, method: str, params: dict) -> dict:
        """Makes RPC query to NEAR network"""
        self.env.add_system_log(f"Making RPC query - Method: {method}, Params: {params}", logging.INFO)
        try:
            response = requests.post(
                self.rpc_url,
                json={
                    "jsonrpc": "2.0",
                    "id": "benevio.dev",
                    "method": "query",
                    "params": {"request_type": method, "finality": "final", **params},
                },
            )
            response.raise_for_status()
            result = response.json()["result"]
            self.env.add_system_log(f"RPC query successful - Result: {result}", logging.INFO)
            return result
        except requests.exceptions.RequestException as e:
            self.env.add_system_log(f"RPC query failed: {str(e)}", logging.ERROR)
            raise

    def _parse_view_result(self, response: dict) -> str:
        """
        Parses the view call result from bytes to string.

        Args:
            response (dict): Raw RPC response containing result bytes

        Returns:
            str: Decoded string value

        Example:
            >>> response = {'result': [34, 115, 101, 99...]}
            >>> result = client._parse_view_result(response)
            >>> print(result)
            >>> # 'secp256k1:3R64TGr9wxtGmXBjgZmEEqCMDycaYSRsrq6hAbTJdk8ZQ6gc3FuyiF5Scw2FPx3evaEfScjiGARN7GVrpXuEZCq3'
        """
        self.env.add_system_log("Parsing view result from bytes", logging.DEBUG)
        try:
            # Convert byte array to string
            result_bytes = bytes(response.get("result", []))
            # Remove quotes if present
            decoded = result_bytes.decode("utf-8").strip('"')
            self.env.add_system_log(f"Successfully parsed view result: {decoded}", logging.INFO)
            return decoded
        except Exception as e:
            self.env.add_system_log(f"Failed to parse view result: {str(e)}", logging.ERROR)
            raise
