import asyncio
from typing import Union
import json
import requests
from typing import NewType
import base64

import secrets
from typing import List, Tuple, Dict, Union, TypedDict, Union

from datetime import datetime, timedelta, timezone

BASE_URL = "https://solver-relay-v2.chaindefuser.com/rpc"
TGAS = 1_000_000_000_000
DEFAULT_ATTACHED_GAS = 100 * TGAS
ONE_NEAR = 1_000_000_000_000_000_000_000_000


ASSET_MAP = {
    'USDC': {
        'token_id': '17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1',
        'omft': 'eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near',
        'decimals': 6,
    },
    'USDT': {
        'token_id': 'nep141:usdt.tether-token.near',
        'decimals': 6,
    },
    'NEAR': {
        'token_id': 'wrap.near',
        'decimals': 24,
    }}


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


TokenAddress = NewType('TokenAddress', str)
TokenQuantity = NewType('TokenQuantity', float)
TokenMap = list[tuple[TokenAddress, TokenQuantity]]
QuoteID = NewType('QuoteID', str)
USDValue = NewType('USDValue', float)
BestQuote = Tuple[QuoteID, USDValue]
Quote = Dict[str, Union[float, TokenAddress]]
QuoteTuples = List[Tuple[List[Quote], BestQuote]]


def get_usdc_quotes(token_to_quantities: TokenMap) -> list:
    # map to get quotes for each token_out_id
    return list(map(lambda token_in: get_quotes([token_in], [
                token_to_quantities[token_in]], get_usdc_token_out_type(token_in)), token_to_quantities.keys()))


def get_usdt_quotes(token_to_quantities: TokenMap) -> list:
    # map to get quotes for each token_out_id
    return list(map(lambda token_in: get_quotes([token_in], [
                token_to_quantities[token_in]], get_usdt_token_out_type(token_in)), token_to_quantities.keys()))


def get_near_account_balance(account_id: str) -> float:
    """
    Get account balance for given NEAR account ID.

    Args:
        account_id: NEAR account ID to query

    Returns:
        float: Account balance in yoctoNEAR
    """
    print(f"fetching account balance for {account_id}")
    response = requests.post(
        "https://rpc.mainnet.fastnear.com",
        headers={"Content-Type": "application/json"},
        json={
            "jsonrpc": "2.0",
            "id": "fastnear",
            "method": "query",
            "params": {
                "request_type": "view_account",
                "finality": "final",
                "account_id": account_id
            }
        }
    )
    print(response.json())
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
        print(f'Error fetching price from {url}: {e}')
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
    print(f'fetching prices from  {url}')

    return fetch_usd_price(url, lambda o: float(o['data']['amount']))


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
    print(f'calling to fetch from  {url}')
    return fetch_usd_price(url, lambda o: float(o[token]['usd']))

def get_quotes(
        token_in_ids: list[str],
        token_quantities: list[str],
        asset_identifier_out: str) -> QuoteTuples:
    quotes = []
    best_usd_value = {"usd_value": 0}

    for token_id, quantity in zip(token_in_ids, token_quantities):
        print(
            f"looping through token_in:{token_id}, {quantity} token out {asset_identifier_out}")
        try:
            response = requests.post(
                f"{BASE_URL}",
                json={
                    "method": "quote",
                    "params": [{
                        "defuse_asset_identifier_in": token_id,
                        "defuse_asset_identifier_out": asset_identifier_out,
                        "exact_amount_in": str(quantity),
                        "min_deadline_ms": 60000
                    }],
                    "id": "dontcare",
                    "jsonrpc": "2.0"
                }
            )
            if response.status_code == 200:
                data = response.json().get("result", {})
                if isinstance(data, list):
                    for quote in data:
                        usd_value = int(quote.get("amount_out", 0))
                        quotes.append({
                            "usd_value": usd_value,  # Assuming amount_out is in USD,
                            "token_in": quote.get("defuse_asset_identifier_in"),
                            "token_out": quote.get("defuse_asset_identifier_out"),
                            "amount_in": quote.get("amount_in"),
                            "amount_out": quote.get("amount_out"),
                            "expiration_time": quote.get("expiration_time")
                        })
                        if usd_value > best_usd_value.get("usd_value"):
                            best_usd_value = {
                                "quote_hash": quote.get("quote_hash"),
                                "amount_in": quote.get("amount_in"),
                                "token_in": quote.get("defuse_asset_identifier_in"),
                                "token_out": quote.get("defuse_asset_identifier_out"),
                                "amount_out": quote.get("amount_out"),
                                "usd_value": usd_value,
                                "expiration_time": quote.get("expiration_time")}
        except requests.RequestException as e:
            print(f"Error fetching quote for token {token_id}: {e}")

    return quotes, best_usd_value


def get_recommended_token_allocations(target_usd_amount: float):
    try:
        params = {
            'targetUsdAmount':target_usd_amount *1000000,
            'tokenBalances': json.dumps({
                "BTC": 0.08,
                "ETH": 0.5,
                "SOL": 4.2,
                "NEAR": 330.42928
            })
        }

        response = requests.get("https://ft-allowance-allocations.hello-d1f.workers.dev/", params=params)
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


# TODO replace this with a MPC call to sign an intent payload
#def sign_quote(quote: dict) -> Commitment:
#    quote_str = json.dumps(quote)
#    account = get_account()
#    signature = 'ed25519:' + \
#        base58.b58encode(account.signer.sign(
#            quote_str.encode('utf-8'))).decode('utf-8')
#    public_key = 'ed25519:' + \
#        base58.b58encode(account.signer.public_key).decode('utf-8')s

def publish_intent(signed_intent):
    """Publishes the signed intent to the solver bus."""
    try:
        rpc_request = {
            "id": "dontcare",
            "jsonrpc": "2.0",
            "method": "publish_intent",
            "params": [signed_intent]
        }
        response = requests.post(
            "https://solver-relay-v2.chaindefuser.com/rpc", json=rpc_request)
    except requests.RequestException as e:
        print(f"Error publishing intent {e}")
    return response.json()

# testing logic that will be encapsulated in swap_near_for_usdc


async def main():

    # one_eth = 1000000000000000000
    # one_sol = 1000000000

    # token_quantities =  {"nep141:wrap.near": 5  * ONE_NEAR, "nep141:sol.omft.near": one_sol, "nep141:eth.omft.near":one_eth}
    # target_usd_amount = 500

    # print("USDT Quotes:", get_usdt_quotes({"nep141:wrap.near": 5  * ONE_NEAR}))
    # print("USDC Quotes:", get_usdc_quotes({"nep141:wrap.near": 5  * ONE_NEAR}))

    # Get the best quotes for swapping some NEAR to USDT
    near_to_swap = 1 * ONE_NEAR
    best_quote = get_usdc_quotes({"nep141:wrap.near": near_to_swap})[0][1]
    print("best quote", best_quote)


    await get_recommended_token_allocations(3000)

    # Create a publish_wnear_intent.json payload for the publish_intent call
    deadline = (datetime.now(timezone.utc) + timedelta(minutes=2)
                ).strftime('%Y-%m-%dT%H:%M:%S.000Z')

    # Generate a random nonce
    nonce_base64 = base64.b64encode(secrets.randbits(
        256).to_bytes(32, byteorder='big')).decode('utf-8')

    referral_fee_amount = str(
        int(best_quote.get("amount_out")) // 100)  # 1% of amount_out
    amount_out_less_fee = str(
        int(best_quote.get("amount_out")) - int(referral_fee_amount))

    # TODO replace this with a MPC call to sign an intent payload

    # This is how you swap with a 1% fee to benevio-labs.near
    #payload = Quote(signer_id=AccountId,
    #                nonce=nonce_base64,
    #                verifying_contract="intents.near",
    #                deadline=deadline,
    #                intents=[{"intent": "token_diff",
    #                          "diff": {best_quote.get("token_in"): "-" + str(best_quote.get("amount_in")),
    #                                   best_quote.get("token_out"): amount_out_less_fee},
    #                          "referral": "benevio-labs.near"},
    #                         {"intent": "transfer",
    #                          "receiver_id": "benevio-labs.near",
    #                          "tokens": {best_quote.get("token_out"): referral_fee_amount,
    #                                     },
    #                          "memo": "referral_fee"}])
    #
    #signed_quote = sign_quote(payload)
    #signed_intent = PublishIntent(signed_data=signed_quote, quote_hashes=[
    #                              best_quote.get("quote_hash")])

    #print(publish_intent(signed_intent))


#asyncio.run(main())
