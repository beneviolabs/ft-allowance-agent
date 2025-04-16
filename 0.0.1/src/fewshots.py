"""
Add examples to guide the LLM when a given tool just isn't working like you want it to.
"""

# multi function call
MULTI_CALL_SAMPLE_1 = [
    {
        "id": "example_msg_1",
        "content": "show me my balance and fetch market prices",
        "role": "example_user",
    },
    {
        "id": "example_msg_2",
        "role": "example_assistant",
        "content": "",
        "tool_calls": [
            {
                "id": "call_1",
                "name": "get_near_account_balance",
                "arguments": {},
            },
            {
                "id": "call_2",
                "name": "fetch_token_prices",
                "arguments": {},
            },
        ],
    },
    {
        "tool_call_id": "call_1",
        "role": "tool",
        "name": "get_near_account_balance",
        "content": "1.9181",
    },
    {
        "tool_call_id": "call_2",
        "role": "tool",
        "name": "fetch_token_prices",
        "content": '{"NEAR": 2.1365, "BTC": 84506.7, "ETH": 1638.99, "SOL": 129.66}',
    },
    {
        "id": "example_msg_3",
        "content": """Your current balance is 1.9181 NEAR.
        The current market prices are:
        * NEAR: 2.1365
        * BTC: 84506.7
        * ETH: 1638.99
        * SOL: 129.66""",
        "role": "example_assistant",
    },
]

# recommend swaps
RECOMMEND_SWAP_SAMPLE_1 = [
    {
        "id": "example_msg_1",
        "content": "How can I reach my growth goal?",
        "role": "example_user",
    },
    {
        "id": "example_msg_2",
        "role": "example_assistant",
        "content": "",
        "tool_calls": [
            {
                "id": "call_1",
                "name": "recommend_token_swaps",
                "arguments": {},
            }
        ],
    },
    {
        "tool_call_id": "call_1",
        "role": "tool",
        "name": "recommend_token_swaps",
        "content": '{"BTC": 0.08743453647560796, "ETH": 0.19493149713493173, "SOL": 0.13243573584689358, "NEAR": 6.2904570943770953}',
    },
    {
        "id": "example_msg_3",
        "content": """
To achieve your growth goal of 300, I recommend the following token swaps for USDC/USDT:
* BTC: 0.08743453647560796
* ETH: 0.19493149713493173
* SOL: 0.13243573584689358
* NEAR: 6.2904570943770953

Would you like me to execute these swaps for you?.""",
        "role": "example_assistant",
    },
]

# different amounts
RECOMMEND_SWAP_SAMPLE_2 = [
    {
        "id": "example_msg_1",
        "content": "are there some ways i can reach my goals",
        "role": "example_user",
    },
    {
        "id": "example_msg_2",
        "role": "example_assistant",
        "content": "",
        "tool_calls": [
            {
                "id": "call_1",
                "name": "recommend_token_swaps",
                "arguments": {},
            }
        ],
    },
    {
        "tool_call_id": "call_1",
        "role": "tool",
        "name": "recommend_token_swaps",
        "content": '{"BTC": 0.1857, "NEAR": 5.191913184}',
    },
    {
        "id": "example_msg_3",
        "content": """
Definitely! Here is one way to achieve your current allowance goal of 300. You could swap these amounts of tokens in your wallet for stable coins:
* BTC: 0.1857
* NEAR: 5.191913184

This will rebalance your portfolio.""",
        "role": "example_assistant",
    },
]


ALL_FEWSHOT_SAMPLES = [
    *MULTI_CALL_SAMPLE_1,
    *RECOMMEND_SWAP_SAMPLE_1,
    *RECOMMEND_SWAP_SAMPLE_2,
]
