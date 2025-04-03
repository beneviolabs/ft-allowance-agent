from datetime import datetime, timedelta
import requests
import logging
from typing import Optional
import aiohttp

from src.models import MpcKey, Intent, IntentActions, PublicKey, SignatureRequest
from py_near.account import Account
from dotenv import load_dotenv
import os
import base64
import base58
import json
logger = logging.getLogger(__name__)


class NearMpcClient:
    def __init__(self, network: str = "testnet"):
        # Load environment variables
        load_dotenv()

        # Validate required environment variables
        self._validate_env_vars()

        self.network = network
        self.rpc_url = f"https://rpc.{network}.fastnear.com"
        self.mpc_signer = "v1.signer-prod.testnet" if network == "testnet" else "v1.signer"
        self._derived_key: Optional[str] = None
        self._account_public_key: Optional[PublicKey] = None

    def _validate_env_vars(self):
        """Validate that all required environment variables are set"""
        required_vars = [
            'AGENT_SECRET_KEY',
            'AGENT_ACCOUNT_ID'
        ]

        missing_vars = [var for var in required_vars if not os.getenv(var)]

        if missing_vars:
            error_msg = f"Missing required environment variables: {', '.join(missing_vars)}"
            logger.error(error_msg)
            raise ValueError(error_msg)

    async def _fetch_latest_block_hash(self) -> str:
        """Fetches the latest block hash from the NEAR network"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    self.rpc_url,
                    json={
                        "jsonrpc": "2.0",
                        "id": "benevio.dev",
                        "method": "block",
                        "params": {
                            "finality": "final"
                        }
                    }
                ) as response:
                    if response.status == 200:
                        data = await response.json()
                        block_hash = data["result"]["header"]["hash"]
                        logger.info(f"Got block hash: {block_hash}")
                        return block_hash
                    else:
                        error_msg = f"RPC request failed with status {response.status}"
                        logger.error(error_msg)
                        raise ValueError(error_msg)

        except Exception as e:
            logger.error(f"Failed to fetch block hash: {str(e)}", exc_info=True)
            raise

    def derive_mpc_key(self, proxy_account_id: str) -> MpcKey:
        """Derives MPC key for given account"""
        logger.debug(f"Deriving MPC key for account: {proxy_account_id}")
        try:
            keys = self._get_public_key(proxy_account_id)
            pk = self._get_full_access_key(keys)
            if not pk:
                raise ValueError("No full access key found")
            args = {
                "predecessor": proxy_account_id,
                "path": pk,
            }
            response = self._query_rpc("call_function", {
                "method_name": "derived_public_key",
                "account_id": self.mpc_signer,
                "args_base64": base64.b64encode(json.dumps(args).encode()).decode(),
            })
            self._derived_key = self._parse_view_result(response)
            logger.info(f"Successfully derived MPC key {self._derived_key}")
            return MpcKey(
                public_key=self._derived_key,
                account_id=proxy_account_id,
            )
        except Exception as e:
            logger.error(f"Failed to derive MPC key: {str(e)}", exc_info=True)
            raise

    def _get_full_access_key(self, keys: list[PublicKey]) -> Optional[PublicKey]:
        """Finds the first full access ED25519 public key from a list of keys."""
        try:
            for key in keys:
                permission = key["access_key"]["permission"]
                public_key = key["public_key"]

                if (permission == "FullAccess" and
                    public_key.startswith("ed25519:")):
                    logger.info(f"Found full access key: {public_key}")
                    self._account_public_key = key["public_key"]
                    return self._account_public_key

            logger.warning("No full access ED25519 key found")
            return None

        except Exception as e:
            logger.error(f"Error parsing keys: {str(e)}", exc_info=True)
            raise

    def _get_public_key(self, account_id: str) -> list[PublicKey]:
        """Returns public keys for a given account"""
        try:
            response = self._query_rpc("view_access_key_list", {"finality": "final", "account_id": account_id})
            if "keys" in response and len(response["keys"]) > 0:
                return response["keys"]
            else:
                raise ValueError(f"No keys found for account {account_id}")
        except Exception as e:
            logger.error(f"Failed to get public key: {str(e)}", exc_info=True)
            raise

    def _get_next_nonce(self, proxy_account_id: str) -> int:
        """Calculate a next nonce for a given account"""

        if self._derived_key is None:
            self.derive_mpc_key(proxy_account_id)
        try:
            logger.debug(f"Using derived key: {self._derived_key}")
            response = self._query_rpc("view_access_key", {"finality": "final", "account_id": proxy_account_id, "public_key": self._derived_key})
            if "nonce" in response:
                return str(response["nonce"] + 10)
            else:
                raise ValueError(f"No nonce found for account {proxy_account_id}")
        except Exception as e:
            logger.error(f"Failed to get next nonce: {str(e)}", exc_info=True)
            raise


    async def request_signature(self, proxy_account_id: str, request: SignatureRequest) -> str:
        """Requests signature for given parameters"""
        logger.debug(f"Requesting signature with: {request.dict()}")
        try:
            result = await self._call_contract(proxy_account_id, request.dict())
            success_value = result.status.get('SuccessValue')
            logger.info(f"Successfully requested signature: {success_value}")
            return self._decode_success_value(success_value)
        except Exception as e:
            logger.error(f"Signature request failed: {str(e)}", exc_info=True)
            raise

    async def _request_intent_signature(self, proxy_account_id: str, intent: Intent, block_hash: str) -> str:
        """Publishes swap intent to Defuse network"""
        logger.debug(f"Publishing swap intent: {intent}")
        try:

            TGAS = 1_000_000_000_000
            DEFAULT_ATTACHED_GAS = 100 * TGAS

            # Create signature request with cleaned intent
            signature_request = SignatureRequest(
                contract_id=intent.verifying_contract,
                args=json.dumps(intent.dict()),
                deposit=str(DEFAULT_ATTACHED_GAS),
                nonce=self._get_next_nonce(proxy_account_id),
                block_hash=block_hash,
                mpc_signer_pk=self._derived_key,
                account_pk_for_mpc=self._account_public_key
            )

            # Request signature
            result = await self.request_signature(proxy_account_id, signature_request)
            return result
        except Exception as e:
            logger.error(f"Failed to publish swap intent: {str(e)}", exc_info=True)
            raise

    async def sign_intent(self,
                    proxy_account_id: str,
                    token_in_address: str,
                    token_out_address: str,
                    token_in_amount: str,
                    token_out_amount: str,
                    quote_hash: str,
                    deadline: str,
                    nonce: str) -> dict:
        """Creates intent object with proper formatting"""
        logger.debug(f"Creating intent for signer: {proxy_account_id}")
        try:

            if self.network != "mainnet":
                logger.error("Intent creation attempted on non-mainnet network")
                raise ValueError("Intent creation is only supported on mainnet")

            token_diffs = [
                IntentActions(
                    intent="token_diff",
                    diff={
                        token_in_address: "-"+token_in_amount,
                        token_out_address: token_out_amount
                    }
                )
            ]

            intent = Intent(
                signer_id=proxy_account_id,
                nonce=nonce,
                verifying_contract="intents.near",
                deadline=deadline,
                intents=token_diffs
            )
            logger.info(f"Successfully created intent  {intent}")

            block_hash = await self._fetch_latest_block_hash()

            signature = await self._request_intent_signature(proxy_account_id, intent, block_hash)
            signature = 'secp256k1:' + signature

            result = {
                "signature": signature,
                "intent": intent.dict(),
                "quote_hash": quote_hash,
                "public_key": self._derived_key
            }
            logger.info(f"Returning result: {result}")
            return result

        except Exception as e:
            logger.error(f"Intent creation failed: {str(e)}", exc_info=True)
            raise

    async def _call_contract(self, proxy_account_id: str, params: dict) -> dict:
        """Signed as the agentic account, this function sends a transaction for an MPC signature request to the user's proxy account."""
        try:
            agent_account = Account(os.environ['AGENT_ACCOUNT_ID'], os.environ['AGENT_SECRET_KEY'])
            await agent_account.startup()

            logger.info(f"Calling contract with params: {params}")
            result = await agent_account.function_call(
                proxy_account_id,
                'request_signature',
                args=params,
                gas=100000000000000,  # 100 TGas
                amount=1,
            )

             # Log receipt outcomes
            logger.info("Transaction Results:")
            logger.info(f"Transaction Outcome: {result.transaction_outcome.status}")

            for idx, receipt in enumerate(result.receipt_outcome):
                logger.info(f"Receipt Outcome {idx}:")
                logger.info(f"  Status: {receipt.status}")
                logger.info(f"  Outcome:")
                logger.info(f"    Logs: {receipt.logs}")
                logger.info(f"    Receipt IDs: {receipt.receipt_ids}")

            if "SuccessValue" not in result.status:
                raise Exception(f"Contract call failed with status: {result.status}")
            return result

        except Exception as e:
            logger.error(f"Contract call failed: {str(e)}", exc_info=True)
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
        logger.debug(f"Decoding Base64 value: {base64_decoded}")

        # Remove quotes if present and decode from Base58
        clean_value = base64_decoded.decode('utf-8').strip('"')
        logger.debug(f"Decoding Base64 value: {clean_value}")
        return clean_value


    def _query_rpc(self, method: str, params: dict) -> dict:
        """Makes RPC query to NEAR network"""
        logger.debug(f"Making RPC query - Method: {method}, Params: {params}")
        try:
            response = requests.post(self.rpc_url, json={
                "jsonrpc": "2.0",
                "id": "benevio.dev",
                "method": "query",
                "params": {
                    "request_type": method,
                    "finality": "final",
                    **params
                }
            })
            response.raise_for_status()
            result = response.json()["result"]
            logger.debug(f"RPC query successful - Result: {result}")
            return result
        except requests.exceptions.RequestException as e:
            logger.error(f"RPC query failed: {str(e)}", exc_info=True)
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
        logger.debug("Parsing view result from bytes")
        try:
            # Convert byte array to string
            result_bytes = bytes(response.get('result', []))
            # Remove quotes if present
            decoded = result_bytes.decode('utf-8').strip('"')
            logger.info(f"Successfully parsed view result: {decoded}")
            return decoded
        except Exception as e:
            logger.error(f"Failed to parse view result: {str(e)}", exc_info=True)
            raise
