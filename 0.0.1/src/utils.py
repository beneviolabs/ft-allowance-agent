import asyncio
import base64
import json
import logging
import os
import secrets
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from typing import Dict, List, NewType, Tuple, TypedDict, Union

import aiohttp
import base58
import requests
from cryptography.hazmat.primitives.asymmetric import ed25519
from dotenv import load_dotenv

# from src.models import SignatureRequest
# from src.client import NearMpcClient

logger = logging.getLogger(__name__)

BASE_URL = "https://solver-relay-v2.chaindefuser.com/rpc"
TGAS = 1_000_000_000_000
DEFAULT_ATTACHED_GAS = 100 * TGAS
ONE_NEAR = 1_000_000_000_000_000_000_000_000


load_dotenv()
# AccountId = os.getenv("ACCOUNT_ID")
# PrivKey = os.getenv("FA_PRIV_KEY")
# if AccountId is None or PrivKey is None:
#    raise EnvironmentError(
#        "ACCOUNT_ID and FA_PRIV_KEY must be set in environment variables")
# acc = Account(AccountId, PrivKey)


def get_account():
    near_provider = near_api.providers.JsonProvider("https://rpc.mainnet.near.org")
    key_pair = near_api.signer.KeyPair(PrivKey)
    signer = near_api.signer.Signer(AccountId, key_pair)
    return near_api.account.Account(near_provider, signer, AccountId)


def yocto_to_near(amount: str) -> float:
    """Convert yoctoNEAR string to NEAR float"""
    return float(Decimal(amount) / ONE_NEAR)


def near_to_yocto(amount: float) -> str:
    """Convert NEAR amount to yoctoNEAR string"""
    return str(int(Decimal(str(amount)) * ONE_NEAR))


def format_token_amount(amount: float, decimals: int) -> str:
    """Format token amount with proper decimals"""
    return str(int(Decimal(str(amount)) * Decimal(str(10**decimals))))


ASSET_MAP = {
    "USDC": {
        "token_id": "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1",
        "omft": "eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near",
        "decimals": 6,
    },
    "USDT": {
        "token_id": "nep141:usdt.tether-token.near",
        "decimals": 6,
    },
    "NEAR": {
        "token_id": "wrap.near",
        "decimals": 24,
    },
}


# TODO refactor to make use of ASSET_MAP
def get_usdc_token_out_type(token_in):
    # usdc address may vary per token_in_id, e.g. for token_in_id:
    # "nep141:eth.omft.near", USDC tokenOut should be
    # "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near"

    usdc_out = "nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"
    if token_in == "nep141:eth.omft.near":
        return "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near"
    elif token_in == "nep141:sol.omft.near":
        return "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near"
    elif token_in == "nep141:wrap.near":
        return usdc_out
    else:
        return usdc_out


def get_usdt_token_out_type(token_in):
    usdt_out = "nep141:usdt.tether-token.near"
    if token_in == "nep141:eth.omft.near":
        return "nep141:eth-0xdac17f958d2ee523a2206206994597c13d831ec7.omft.near"
    elif token_in == "nep141:sol.omft.near":
        return "nep141:eth-0xdac17f958d2ee523a2206206994597c13d831ec7.omft.near"
    elif token_in == "nep141:wrap.near":
        return usdt_out
    else:
        return usdt_out


TokenAddress = NewType("TokenAddress", str)
TokenQuantity = NewType("TokenQuantity", float)
TokenMap = list[tuple[TokenAddress, TokenQuantity]]
QuoteID = NewType("QuoteID", str)
USDValue = NewType("USDValue", float)
BestQuote = Tuple[QuoteID, USDValue]
Quote = Dict[str, Union[float, TokenAddress]]
QuoteTuples = List[Tuple[List[Quote], BestQuote]]


def get_usdc_quotes(token_to_quantities: TokenMap) -> list:
    # map to get quotes for each token_out_id
    return list(
        map(
            lambda token_in: get_quotes(
                [token_in],
                [token_to_quantities[token_in]],
                get_usdc_token_out_type(token_in),
            ),
            token_to_quantities.keys(),
        )
    )


async def get_usdt_quotes(token_to_quantities: TokenMap) -> list:
    # Create a list of coroutines - each one is a call to get_quotes()
    coroutines = [
        get_quotes(
            [token_in],
            [token_to_quantities[token_in]],
            get_usdt_token_out_type(token_in),
        )
        for token_in in token_to_quantities.keys()
    ]
    return await asyncio.gather(*coroutines)


def get_near_account_balance(account_id: str) -> float:
    """
    Get account balance for given NEAR account ID.

    Args:
        account_id: NEAR account ID to query

    Returns:
        float: Account balance in yoctoNEAR
    """
    response = requests.post(
        "https://rpc.mainnet.fastnear.com",
        headers={"Content-Type": "application/json"},
        json={
            "jsonrpc": "2.0",
            "id": "benevio.dev",
            "method": "query",
            "params": {
                "request_type": "view_account",
                "finality": "final",
                "account_id": account_id,
            },
        },
    )
    return response.json()["result"]["amount"]


def fetch_usd_price(url: str, parse_price: callable) -> Union[float, bool]:
    """
    Fetches USD price from API endpoint and parses response.

    Args:
        url: API endpoint URL
        parse_price: Function to parse price from response JSON

    Returns:
        float: Parsed price if successful
        bool: False if request fails
    """
    try:
        response = requests.get(url)
        response.raise_for_status()
        data = response.json()
        return parse_price(data)
    except requests.RequestException as e:
        print(f"Error fetching price from {url}: {e}")
        return False


def fetch_coinbase(token: str) -> Union[float, bool]:
    """
    Fetches USD price for a token from Coinbase API.

    Args:
        token: Token symbol (e.g. 'BTC', 'ETH')

    Returns:
        float: USD price if successful
        bool: False if request fails
    """
    url = f"https://api.coinbase.com/v2/prices/{token}-USD/buy"
    print(f"fetching prices from  {url}")

    return fetch_usd_price(url, lambda o: float(o["data"]["amount"]))


def fetch_coingecko(token: str) -> Union[float, bool]:
    """
    Fetches USD price for a token from CoinGecko API.

    Args:
        token: Token ID (e.g. 'bitcoin', 'ethereum')

    Returns:
        float: USD price if successful
        bool: False if request fails
    """
    url = f"https://api.coingecko.com/api/v3/simple/price?ids={token}&vs_currencies=usd"
    print(f"calling to fetch from  {url}")
    return fetch_usd_price(url, lambda o: float(o[token]["usd"]))


async def get_quotes(
    token_in_ids: list[str], token_quantities: list[str], asset_identifier_out: str
) -> QuoteTuples:
    quotes = []
    best_usd_value = {"usd_value": 0}

    async with aiohttp.ClientSession() as session:
        for token_id, quantity in zip(token_in_ids, token_quantities):
            print(
                f"Getting quote for token_in:{token_id}, {quantity} token out {asset_identifier_out}"
            )
            try:
                async with session.post(
                    BASE_URL,
                    json={
                        "method": "quote",
                        "params": [
                            {
                                "defuse_asset_identifier_in": token_id,
                                "defuse_asset_identifier_out": asset_identifier_out,
                                "exact_amount_in": str(quantity),
                                "min_deadline_ms": 60000,
                            }
                        ],
                        "id": "benevio.dev",
                        "jsonrpc": "2.0",
                    },
                ) as response:
                    print(f"Response: {response}")
                    if response.status == 200:
                        data = await response.json()
                        result = data.get("result", {})
                        if isinstance(result, list):
                            for quote in result:
                                usd_value = int(quote.get("amount_out", 0))
                                quotes.append(
                                    {
                                        "usd_value": usd_value,  # TODO assumes amount_out is in USD,
                                        "token_in": quote.get(
                                            "defuse_asset_identifier_in"
                                        ),
                                        "token_out": quote.get(
                                            "defuse_asset_identifier_out"
                                        ),
                                        "amount_in": quote.get("amount_in"),
                                        "amount_out": quote.get("amount_out"),
                                        "expiration_time": quote.get("expiration_time"),
                                    }
                                )
                                if usd_value > best_usd_value.get("usd_value"):
                                    best_usd_value = {
                                        "quote_hash": quote.get("quote_hash"),
                                        "amount_in": quote.get("amount_in"),
                                        "token_in": quote.get(
                                            "defuse_asset_identifier_in"
                                        ),
                                        "token_out": quote.get(
                                            "defuse_asset_identifier_out"
                                        ),
                                        "amount_out": quote.get("amount_out"),
                                        "usd_value": usd_value,
                                        "expiration_time": quote.get("expiration_time"),
                                    }
            except Exception as e:
                print(f"Error fetching quote for token {token_id}: {e}")

    return quotes, best_usd_value


def get_recommended_token_allocations(target_usd_amount: float):
    try:
        params = {
            "targetUsdAmount": target_usd_amount * 1000000,
            "tokenBalances": json.dumps(
                {"BTC": 0.08, "ETH": 0.5, "SOL": 4.2, "NEAR": 330.42928}
            ),
        }

        swap_service_url = os.environ.get("swap_allocations_worker")
        response = requests.get(swap_service_url, params=params)
        print(response.json())
        return response.json() if response.status_code == 200 else None
    except requests.RequestException as e:
        print(f"Error fetching allocations: {e}")
        return None


class AcceptQuote(TypedDict):
    nonce: str
    recipient: str
    message: str


class Commitment(TypedDict):
    standard: str
    payload: Union[AcceptQuote, str]
    signature: str
    public_key: str


class PublishIntent(TypedDict):
    signed_data: Commitment
    quote_hashes: List[str] = []


class Intent(TypedDict):
    intent: str
    diff: Dict[str, str]


class Quote(TypedDict):
    nonce: str
    signer_id: str
    verifying_contract: str
    deadline: str
    intents: List[Intent]


def sign_quote(quote: dict) -> Commitment:
    print(f"Signing quote: {quote}")
    quote_str = json.dumps(quote)
    account = get_account()
    signature = "ed25519:" + base58.b58encode(
        account.signer.sign(quote_str.encode("utf-8"))
    ).decode("utf-8")
    public_key = "ed25519:" + base58.b58encode(account.signer.public_key).decode(
        "utf-8"
    )
    print(f"Account signer public key: {public_key}")
    try:
        check_pub_key = ed25519.Ed25519PublicKey.from_public_bytes(
            account.signer.public_key
        )
        check_pub_key.verify(
            base58.b58decode(signature[8:]), json.dumps(quote).encode("utf-8")
        )
        print("Signature is valid. {signature}")
    except ed25519.InvalidSignature:
        print("Invalid signature.")

    return Commitment(
        standard="raw_ed25519",
        payload=quote_str,
        signature=signature,
        public_key=public_key,
    )


def publish_intent(signed_intent):
    print(f"Publishing intent: {json.dumps(signed_intent)}")
    """Publishes the signed intent to the solver bus."""
    try:
        rpc_request = {
            "id": "benevio.dev",
            "jsonrpc": "2.0",
            "method": "publish_intent",
            "params": [signed_intent],
        }
        response = requests.post(
            "https://solver-relay-v2.chaindefuser.com/rpc", json=rpc_request
        )
    except requests.RequestException as e:
        print(f"Error publishing intent {e}")
    return response.json()


def old_demo_of_manual_swaps():
    # Create a publish_wnear_intent.json payload for the publish_intent call
    deadline = (datetime.now(timezone.utc) + timedelta(minutes=2)).strftime(
        "%Y-%m-%dT%H:%M:%S.000Z"
    )

    # Get Quotes for USDT
    # best_quote = await get_usdt_quotes({"nep141:wrap.near": 1 * ONE_NEAR})
    best_quote = best_quote[0][1]
    print("Best USDT Quote:", best_quote)

    # Generate a random nonce
    nonce_base64 = base64.b64encode(
        secrets.randbits(256).to_bytes(32, byteorder="big")
    ).decode("utf-8")

    # payload = Quote(signer_id=AccountId,
    #                nonce=nonce_base64,
    #                verifying_contract="intents.near",
    #                deadline=deadline,
    #                intents=[{"intent": "token_diff",
    #                          "diff": {best_quote.get("token_in"): "-" + str(best_quote.get("amount_in")),
    #                                   best_quote.get("token_out"): str(best_quote.get("amount_out"))},
    #                          "referral": "benevio-labs.near"},
    #                       ])
    #
    # publish_payload = sign_quote(payload)

    # client = NearMpcClient(network="mainnet")

    # client.derive_mpc_key("agent.charleslavon.near")

    # signed_intent = await client.sign_intent("agent.charleslavon.near", best_quote["token_in"], best_quote["token_out"], best_quote["amount_in"], best_quote["amount_out"], best_quote["quote_hash"], best_quote["expiration_time"], nonce_base64)
    ##
    # publish_payload = Commitment(
    #    standard="raw_ed25519",
    #    payload=json.dumps(signed_intent.get("intent")),
    #    signature=signed_intent.get("signature"),
    #    public_key=signed_intent.get("public_key")
    # )


#
# publish_payload = PublishIntent(signed_data=publish_payload, quote_hashes=[
#                                    best_quote.get("quote_hash")])

# print(publish_intent(publish_payload))


async def demo_quote():
    """Demonstrate OneClick API quote functionality"""
    from .client import NearMpcClient

    client = NearMpcClient(network="mainnet")
    dry = True
    try:
        quotes = await client.get_stablecoin_quotes(
            "nep141:wrap.near",
            "50000000000000000000000",
            "agent.charleslavon.near",
            dry,
        )
        logger.debug(f"Quote details: {quotes}")
        # await client.oneclickapi.check_transaction_status("7d1eaa39006bcec14a040cdd10f876be458dc222e0dced46057bd0a036c36f08")

        # supported_tokens = await client.oneclickapi.get_supported_tokens()
        # print(f"Supported tokens: {supported_tokens}")

        if not dry:
            deposit_address = quotes["USDC"].quote.get("depositAddress")
            print(f"Deposit address: {deposit_address}")

            status = await client.oneclickapi.check_transaction_status(deposit_address)
            print(f"Transaction status: {status}")

    finally:
        await client.close()
