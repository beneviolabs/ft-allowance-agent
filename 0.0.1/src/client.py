from datetime import datetime, timedelta
import requests
import logging
from typing import Optional

from models import MpcKey, Intent, PublicKey, SignatureRequest
from near_api.account import Account
from near_api.signer import KeyPair
import os
import base64
import json
logger = logging.getLogger(__name__)

class NearMpcClient:
    def __init__(self, network: str = "testnet"):
        self.network = network
        self.rpc_url = f"https://rpc.{network}.fastnear.com"
        self.mpc_signer = "v1.signer-prod.testnet" if network == "testnet" else "v1.signer"

    def derive_mpc_key(self, proxy_account_id: str) -> MpcKey:
        """Derives MPC key for given account"""
        logger.debug(f"Deriving MPC key for account: {proxy_account_id}")
        try:
            keys = self._get_public_key(proxy_account_id)
            keyinfo = self._get_full_access_key(keys)
            if not keyinfo:
                raise ValueError("No full access key found")
            args = {
                "predecessor": proxy_account_id,
                "path": keyinfo["public_key"],
            }
            response = self._query_rpc("call_function", {
                "method_name": "derived_public_key",
                "account_id": self.mpc_signer,
                "args_base64": base64.b64encode(json.dumps(args).encode()).decode(),
            })
            derived_key = self._parse_view_result(response)
            logger.info(f"Successfully derived MPC key {derived_key}")
            return MpcKey(
                public_key=derived_key,
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
                    return key

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

    # TODO test me in utils.py
    def request_signature(self, request: SignatureRequest) -> str:
        """Requests signature for given parameters"""
        logger.debug(f"Requesting signature for contract: {request.contract_id}")
        try:
            result = self._call_contract("request_signature", request.dict())
            logger.info(f"Successfully requested signature: {result}")
            return result
        except Exception as e:
            logger.error(f"Signature request failed: {str(e)}", exc_info=True)
            raise

    # TODO test me in utils.py
    def create_intent(self,
                     signer_id: str,
                     token_diffs: dict[str, str],
                     deadline: Optional[datetime] = None) -> Intent:
        """Creates intent object with proper formatting"""
        logger.debug(f"Creating intent for signer: {signer_id}")
        try:
            if not deadline:
                deadline = datetime.utcnow() + timedelta(minutes=2)
            if self.network != "mainnet":
                logger.error("Intent creation attempted on non-mainnet network")
                raise ValueError("Intent creation is only supported on mainnet")

            intent = Intent(
                signer_id=signer_id,
                nonce=self._get_next_nonce(signer_id),
                verifying_contract="intents.near",
                deadline=deadline.isoformat() + "Z",
                token_diffs=token_diffs
            )
            logger.info(f"Successfully created intent for {signer_id}")
            return intent
        except Exception as e:
            logger.error(f"Intent creation failed: {str(e)}", exc_info=True)
            raise

    # TODO test me in utils.py
    def _call_contract(self, params: dict) -> dict:
        """Signed as the agentic account, this function sends a transaction for an MPC signature request to the user's proxy account."""
        try:
            agent_signer = KeyPair(
                os.environ['agent_pub_key'],
                os.environ['agent_private_key']
            )
            agent_account = Account(
                self.rpc_url,
                os.environ['agent_account_id'],
                agent_signer
            )

            result = agent_account.function_call(
                params['proxy_account_id'],
                'request_signature',
                params,
                gas=50000000000000, # 50 TGas
                deposit=0
            )
            logger.info(f"Contract call successful: {result}")
            return result

        except Exception as e:
            logger.error(f"Contract call failed: {str(e)}", exc_info=True)
            raise


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
