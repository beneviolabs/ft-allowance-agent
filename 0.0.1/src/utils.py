import asyncio
import json
import os
from decimal import Decimal
from typing import Dict, List, NewType, Tuple, TypedDict, Union

import aiohttp
import requests
from dotenv import load_dotenv

from .log_adapter import LoggerAdapter

# from src.models import SignatureRequest
# from src.client import NearMpcClient

logger = LoggerAdapter()

BASE_URL = "https://solver-relay-v2.chaindefuser.com/rpc"
TGAS = 1_000_000_000_000
DEFAULT_ATTACHED_GAS = 100 * TGAS
ONE_NEAR = 1_000_000_000_000_000_000_000_000

load_dotenv()


def set_environment(env):
    """Set environment for logging"""
    global logger
    logger = LoggerAdapter(env)


def yocto_to_near(amount: str) -> float:
    """Convert yoctoNEAR string to NEAR float"""
    return float(Decimal(amount) / ONE_NEAR)


def near_to_yocto(amount: str) -> int:
    """Convert NEAR float to yoctoNEAR string"""
    return int(Decimal(amount) * ONE_NEAR)


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


def get_near_account_balance(network: str, account_id: str) -> float:
    """
    Get account balance for given NEAR account ID.

    Args:
        account_id: NEAR account ID to query

    Returns:
        float: Account balance in yoctoNEAR
    """
    response = requests.post(
        f"https://rpc.{network}.fastnear.com",
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


def publish_transaction(network: str, signed_transaction: str) -> dict:
    """
    Submit a signed transaction to the NEAR network.

    Args:
        network: NEAR network to use (mainnet, testnet)
        signed_transaction: Base64 encoded signed transaction

    Returns:
        dict: Transaction result containing hash and other details

    Raises:
        requests.RequestException: If the RPC request fails
    """
    response = requests.post(
        f"https://rpc.{network}.fastnear.com",
        headers={"Content-Type": "application/json"},
        json={
            "jsonrpc": "2.0",
            "id": "benevio.dev",
            "method": "send_tx",
            "params": [signed_transaction],
        },
    )

    if response.status_code != 200:
        raise requests.RequestException(
            f"Failed to submit transaction: {response.text}"
        )

    result = response.json()
    if "error" in result:
        raise requests.RequestException(f"Transaction failed: {result['error']}")

    return result["result"]


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


def build_deposit_and_transfer_actions(
    token_in_address: str, amount_in: str, deposit_address: str
) -> str:
    """
    Build JSON string of actions for wrapping NEAR and transferring to intents.near

    Args:
        amount_in: Amount of NEAR to wrap and transfer (in yoctoNEAR)
        deposit_address: Destination address for the transfer
        token_in_address: Address of the swap from token.

    Returns:
        str: JSON string containing the actions array
    """
    deposit_action = None

    # The manner in which to execute a swap depends on the token_in and token_out types. For Near to USDC/USDT, one must call wrap.near with two actions: deposit and ft_transfer_call with the msg param of stringified JSON containing {"receiver_id": "depositAddress"}. See https://nearblocks.io/txns/AHzB4wWyvrB9bTQByRjsDexY7EqPvm3rfFxmudBZ2gFr#execution
    if token_in_address == "wrap.near":
        deposit_action = {
            "type": "FunctionCall",
            "method_name": "near_deposit",
            "deposit": str(amount_in),
            "gas": "50000000000000",
            "args": {},
        }
    else:
        # Handle other contract addresses, e.g. ETH, SOL
        raise ValueError(f"Unsupported contract address: {token_in_address}")

    actions = [
        deposit_action,
        {
            "type": "FunctionCall",
            "method_name": "ft_transfer_call",
            "args": {
                "receiver_id": "intents.near",
                "amount": str(amount_in),
                "msg": json.dumps({"receiver_id": deposit_address}),
            },
            "gas": "50000000000000",
            "deposit": "1",
        },
    ]
    return json.dumps(actions)


def usd_to_base6(amount: float) -> str:
    """
    Convert USD amount to base6 string.

    Args:
        amount: Amount in USD

    Returns:
        str: Amount in base6 format
    """
    return str(int(amount * 100_000))


def get_recommended_token_allocations(
    target_usd_amount: float, tokenBalances: dict
) -> Union[dict, None]:
    logger.debug(f"Target USD amount: {target_usd_amount}")
    target_usd_amount = usd_to_base6(target_usd_amount)
    logger.debug(f"Target USD amount: {target_usd_amount}")

    try:
        params = {
            "targetUsdAmount": target_usd_amount,
            "tokenBalances": json.dumps(tokenBalances),
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


async def demo_quote():
    """Demonstrate OneClick API quote functionality"""
    from .client import NearMpcClient

    client = NearMpcClient(network="mainnet")
    dry = True
    try:
        quotes = await client.get_stablecoin_quotes(
            "nep141:wrap.near",
            "500000000000000000000000",
            "agent.charleslavon.near",
            dry,
        )

        depopsitAddress = quotes["USDC"].quote.get("depositAddress")
        amount_in = quotes["USDC"].quoteRequest.get("amount")
        print(f"Amount in: {amount_in}")
        print(f"Deposit address: {depopsitAddress}")

        msg = {"receiver_id": depopsitAddress}

        actions = [
            {
                "type": "FunctionCall",
                "method_name": "near_deposit",
                "deposit": str(amount_in),
                "gas": "50000000000000",
                "args": {},
            }
        ]

        if not dry:
            actions.append(
                {
                    "type": "FunctionCall",
                    "method_name": "ft_transfer_call",
                    "deposit": "1",
                    "args": {
                        "receiver_id": "intents.near",
                        "amount": str(amount_in),
                        "msg": json.dumps(msg),
                    },
                    "gas": "50000000000000",
                }
            )

        actions_json = json.dumps(actions)

        logger.debug(f"request payload: {actions_json}")

        siggy = await client._request_multi_action_signature(
            "wrap.near", actions_json, "agent.charleslavon.near"
        )

        print(f"Signature: {siggy}")

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
