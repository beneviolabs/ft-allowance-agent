"""
Add examples to guide the LLM when a given tool just isn't working like you want it to.
"""

# both fields are set
GOALS_SAMPLE_1 = [
    {
        "id": "example_msg_1",
        "content": "what are my goals",
        "role": "example_user",
    },
    {
        "role": "example_assistant",
        "content": {
            "tool_calls": [
                {
                    "id": "call_1",
                    "name": "get_goals",
                    "arguments": {},
                }
            ]
        },
    },
    {
        "tool_call_id": "call_1",
        "role": "tool",
        "name": "get_goals",
        "content": '{"growth": 200, "allowance": 400}',
    },
    {
        "id": "example_msg_2",
        "content": """Your current goals are:
        * Growth goal: 200
        * Allowance goal: 400 stablecoins
        Let me know if you'd like to review or adjust these goals!
        """,
        "role": "example_assistant",
    },
]

GOALS_SAMPLE_2 = [
    {
        "id": "example_msg_3",
        "content": "show me my allowance goal",
        "role": "example_user",
    },
    {
        "role": "example_assistant",
        "content": {
            "tool_calls": [
                {
                    "id": "call_2",
                    "name": "get_goals",
                    "arguments": {},
                }
            ]
        },
    },
    {
        "tool_call_id": "call_2",
        "role": "tool",
        "name": "get_goals",
        "content": '{"growth": "The user hasn\'t set a growth goal yet. Prompt them to provide one.", "allowance": 400}',
    },
    {
        "id": "example_msg_4",
        "content": "It looks like you haven't set a growth goal for your portfolio yet. Would you like to set one now? Additionally, I see that you have set an allowance goal of 400",
        "role": "example_assistant",
    },
]

# no goals are set
GOALS_SAMPLE_3 = [
    {
        "id": "example_msg_5",
        "content": "what are my goals",
        "role": "example_user",
    },
    {
        "role": "example_assistant",
        "content": {
            "tool_calls": [
                {
                    "id": "call_4",
                    "name": "get_goals",
                    "arguments": {},
                }
            ]
        },
    },
    {
        "tool_call_id": "call_4",
        "role": "tool",
        "name": "get_goals",
        "content": '{"growth": "The user hasn\'t set a growth goal yet. Prompt them to provide one.", "allowance": "The user hasn\'t set a allowance goal yet. Prompt them to provide one."}',
    },
    {
        "id": "example_msg_6",
        "content": '{"growth": null, "allowance": null}',
        "role": "example_assistant",
    },
]

# multi function call
# TODO: doesn't work, might be a library issue?
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
# TODO: add save_goals multi call example

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
To achieve your growth goal of 300, I recommend the following token swaps:
* BTC: 0.08743453647560796
* ETH: 0.19493149713493173
* SOL: 0.13243573584689358
* NEAR: 6.2904570943770953

Would you like me to execute these swaps for you?.""",
        "role": "example_assistant",
    },
]

# different wordings
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
