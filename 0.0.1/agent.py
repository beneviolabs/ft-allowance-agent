import inspect
import json
import logging
import re
import traceback
import typing
from types import SimpleNamespace

from src.client import NearMpcClient
from src.fewshots import ALL_FEWSHOT_SAMPLES
from src.utils import (
    fetch_coinbase,
    fetch_coingecko,
    get_near_account_balance,
    get_recommended_token_allocations,
    yocto_to_near,
)

from nearai.agents.environment import ChatCompletionMessageToolCall, Environment

NEAR_ID_REGEX = re.compile(r"^[a-z0-9._-]+\.near$")

DIVVY_GOALS = set(["allowance", "growth"])
DivvyGoalType = typing.Literal["allowance"] | typing.Literal["growth"]


class Agent:
    def __init__(self, env: Environment):
        self.env = env
        self._allowance_goal = None
        self.prices = None
        self.recommended_tokens = None
        self._near_account_id = None
        self._growth_goal = None
        self.near_account_balance = None
        self._client = NearMpcClient(network=env.env_vars["network"])
        tool_registry = self.env.get_tool_registry()
        tool_registry.register_tool(self.recommend_token_swaps)
        tool_registry.register_tool(self.get_near_account_id)
        tool_registry.register_tool(self.save_near_account_id)
        tool_registry.register_tool(self.get_goals)
        tool_registry.register_tool(self.save_goal)
        tool_registry.register_tool(self.get_near_account_balance)
        tool_registry.register_tool(self.fetch_token_prices)

        # hack to disallow some builtin tools from the library. Might be fragile to future changes in which case we can blacklist them in the prompt
        del tool_registry.tools["write_file"]
        del tool_registry.tools["read_file"]
        del tool_registry.tools["list_files"]
        del tool_registry.tools["exec_command"]
        del tool_registry.tools["query_vector_store"]
        del tool_registry.tools["request_user_input"]

    @property
    def near_account_id(self) -> str | None:
        if self._near_account_id is None:
            self._near_account_id = self.env.read_file("near_id.txt")
        return self._near_account_id

    @property
    def allowance_goal(self) -> int | None:
        if self._allowance_goal is None:
            stored_goal = self.env.read_file("allowance_goal.txt")
            self.env.add_system_log(
                f"Found stored allowance goal: {stored_goal}", logging.DEBUG
            )
            if stored_goal:
                self._allowance_goal = int(stored_goal)
        return self._allowance_goal

    @property
    def growth_goal(self) -> int | None:
        if self._growth_goal is None:
            stored_goal = self.env.read_file("growth_goal.txt")
            self.env.add_system_log(
                f"Found stored growth goal: {stored_goal}", logging.DEBUG
            )
            if stored_goal:
                self._growth_goal = int(stored_goal)
        return self._growth_goal

    def run(self):
        # A system message guides an agent to solve specific tasks.
        prompt = {
            "role": "system",
            # Set example responses for this in the fewshots.py
            "content": """You are Divvy, a financial assistant that helps users manage and grow their crypto portfolio.
Your user is a crypto beginner who is looking to set up a portfolio and achieve their financial goals.
Your context will be populated by tool call results to help you respond to the user.

-Capabilities-
You can show user account details such as their balance and their NEAR account ID.
You can provide the real-time current market prices of crypto tokens in the users wallet.
You can allow the user to set growth and allowance goals on their portfolio.
You are capable of personalized recommendations for token swaps to achieve the user's allowance goal in USDC.
You can fetch the NEAR account balance of the user.

You must follow the following instructions:

-Instructions-
* Be polite and helpful to the user.
* When introducing yourself, provide a brief description of what your purpose is.
* Tell the user if you don't support a capability. Do NOT make up or provide false information or figures.
* Do not use figures or function call results from preceding messages to generate responses.
* Be very precise with the numbers you parse from the tool call results, do not add or remove any digits.
""",
        }

        # Use the model set in the metadata to generate a response
        result = None
        tools = self.env.get_tool_registry().get_all_tool_definitions()

        self.env.add_system_log(
            f"Checking whether tool calls are needed from one of: {[t['function']['name'] for t in tools]}",
            logging.DEBUG,
        )

        user_query = self.env.get_last_message()

        tools_plan = self._get_tool_call_plan(user_query)
        self.env.add_system_log(
            f"Should call tools: {tools_plan}",
            logging.DEBUG,
        )

        if tools_plan.message == "noop":
            result = self.env.completion([prompt, user_query])

        elif tools_plan.tool_calls and len(tools_plan.tool_calls) > 0:
            tool_call_results = self._handle_tool_calls(tools_plan.tool_calls)
            if len(tool_call_results) > 0:
                self.env.add_system_log(
                    f"Got tool call results: {tool_call_results}", logging.DEBUG
                )

                context = (
                    [prompt] + ALL_FEWSHOT_SAMPLES + [user_query] + tool_call_results
                )
                result = self.env.completion(context)

                self.env.add_system_log(
                    f"Got completion for tool call with results: {result}. \n --- \n Context: {context}",
                    logging.DEBUG,
                )
        else:
            self.env.add_reply(
                "I had trouble understanding that. Could you please rephrase your question?"
            )

        if result:
            self.env.add_reply(result)

        # Give the prompt back to the user
        self.env.request_user_input()

    @staticmethod
    def _to_function_response(
        tool_call_id: str, function_name: str, value: typing.Any
    ) -> typing.Dict:
        """
        Use to tell the LLM the result from a function call in a structured way
        """
        return {
            "tool_call_id": tool_call_id,
            "role": "tool",
            "name": function_name,
            "content": json.dumps(value),
        }

    def _get_tool_call_plan(self, user_query: str) -> SimpleNamespace:
        """
        Run a completion against the LLM to get a plan of action for the tool calls
        """
        prompt = {
            "role": "system",
            "content": """
You are a Tool Planner for Divvy, a crypto financial assistant.
Your job is to analyze the user's request and decide which tools should be called to fulfill it.
You will NOT generate any friendly language or explanations for the user. Your only job is to return a list of tool calls based on the user's intent.

-Rules-
1. If no tool call is needed, return the string "noop".
2. Your capabilities are facilitated by the functions/tools you have been given. Do not make up any tools.
3. Only call tools needed to fulfill the request — no more, no less.
4. If multiple steps need to be taken to fulfill the request, generate multiple tool calls and in the correct order.
5. Be very precise with the numbers you parse from the tool call args, do not add or remove any digits.
6. You must return structured tool call results in the LLM's native format. Do not use the format from the examples below.

-Examples-

--Example 1--
User: "What is my NEAR account balance?"
Output:
{
    "id": "example_msg_1",
    "role": "example_assistant",
    "content": "",
    "tool_calls": [
        {
            "id": "call_1",
            "name": "get_near_account_balance",
            "arguments": {},
        },
    ],
}

--Example 2--
User: "Show me my balance and fetch token market prices"
Output:
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
}

--Example 3--
User: "How can I realize my allowance goal?" OR "What swaps would you recommend?" OR "Which tokens should I swap?" OR "How do I reach my goal?"
Output:
{
    "id": "example_msg_3",
    "role": "example_assistant",
    "content": "",
    "tool_calls": [
        {
            "id": "call_1",
            "name": "recommend_token_swaps",
            "arguments": {},
        },
    ],
}

--Example 4--
User: "Hello."
Output: "noop"
""",
        }
        tools = self.env.get_tool_registry().get_all_tool_definitions()
        return self.env.completion_and_get_tools_calls(
            [prompt, user_query], tools=tools
        )

    def _get_tool_name(self) -> str:
        """Return the function name of the calling function as the tool name"""
        return inspect.stack()[1][3]

    def get_near_account_id(self) -> str | None:
        """Get the NEAR account ID of the user"""
        if not self.near_account_id:
            self.env.add_reply(
                "There is no NEAR account ID right now. Please provide one.",
                message_type="system",
            )
            return (
                "The user hasn't provided a NEAR account ID yet. Ask them to provide one.",
            )

        return self.near_account_id

    def get_near_account_balance(self) -> str | float:
        """
        Fetch and return the NEAR token balance for a user's account.

        This function should be called when:
        1. The user asks about their NEAR balance
        2. When the user asks questions like:
        - "What's my NEAR balance?"
        - "How much NEAR do I have?"

        Requirements:
        - A valid NEAR account ID must be set (self.near_account_id)
        - If no account ID is set, this will prompt the user to provide one

        Returns:
            List[Dict]: A function response containing the balance:
            [{
                'role': 'function',
                'name': 'get_near_account_balance',
                'content': '1000000000000000000000000'  # Balance in yoctoNEAR
            }]

        Side effects:
        - Sets self.near_account_balance with the human-readable NEAR amount
        - Adds system messages to guide the conversation flow
        - Prompts for account ID if missing
        """
        balance = None
        if self.near_account_id:
            balance = get_near_account_balance(self.near_account_id)
            if balance:
                self.near_account_balance = yocto_to_near(balance)
            self.env.add_reply("Found the user's balance", message_type="system")
        else:
            return "Unable to fetch balance because user hasn't provided NEAR account ID. Ask them to provide it."

        return balance

    # IMPROVE: this function can be parameterized to only query prices for tokens user specifies and fetch all if there's no param value
    def fetch_token_prices(self):
        """Fetch the real-time market prices of the tokens in a user's wallet (e.g. NEAR, BTC, ETH, SOL)"""
        self.env.add_reply(
            "Fetching the current prices of the tokens in your wallet..."
        )
        near_price = fetch_coinbase("near")
        near_price = (
            fetch_coingecko("near") if isinstance(near_price, bool) else near_price
        )

        btc_price = fetch_coinbase("btc")
        btc_price = fetch_coingecko("btc") if isinstance(btc_price, bool) else btc_price

        eth_price = fetch_coinbase("eth")
        eth_price = fetch_coingecko("eth") if isinstance(eth_price, bool) else eth_price

        sol_price = fetch_coinbase("sol")
        sol_price = fetch_coingecko("sol") if isinstance(sol_price, bool) else sol_price
        self.prices = {
            "NEAR": near_price,
            "BTC": btc_price,
            "ETH": eth_price,
            "SOL": sol_price,
        }

        return self.prices

    def get_goals(self):
        """Return the goals (growth or allowance) that the user has set for their portfolio."""
        goals = {}
        for type_, goal in [
            ("growth", self.growth_goal),
            ("allowance", self.allowance_goal),
        ]:
            if not goal:
                goals[type_] = (
                    f"The user hasn't set a {type_} goal yet. Prompt them to provide one."
                )
            else:
                goals[type_] = goal

        return goals

    def _handle_tool_calls(
        self, tool_calls: typing.List[ChatCompletionMessageToolCall]
    ) -> typing.List[typing.Dict]:
        """
        Execute the tool calls and return a result for the LLM to process.
        This is designed after nearai.agents.environment.Environment._handle_tool_calls
        which we don't use because it doesn't expose the tool call results to us
        yet we need to pass them to the LLM for further processing.
        """
        results = []
        for tool_call in tool_calls:
            tool = self.env.get_tool_registry().get_tool(tool_call.function.name)
            if not tool:
                self.env.add_system_log(
                    f"Tool '{tool_call.function.name}' not found in the tool registry.",
                    logging.WARNING,
                )
                continue
            args = json.loads(tool_call.function.arguments)

            try:
                results.append(
                    self._to_function_response(
                        tool_call.id, tool_call.function.name, tool(**args)
                    )
                )
            except Exception as e:
                e_tback = "".join(traceback.format_exception(e))
                error_message = (
                    f"Error calling tool {tool_call.function.name}: {e_tback}"
                )
                self.env.add_system_log(error_message, level=logging.ERROR)
                results.append(
                    self._to_function_response(
                        tool_call.id,
                        tool_call.function.name,
                        "Tell the user a server error occurred while processing the request and to try again later.",
                    )
                )
        return results

    def recommend_token_swaps(self) -> str | None:
        """
        Help the user achieve their goals (allowance, growth) by generating personalized token swap recommendations.
        """
        if self.allowance_goal is None:
            return (
                "The user hasn't set an allowance goal yet. Prompt them to provide one."
            )

        if self.recommended_tokens is None:
            self.env.add_reply(
                "Considering your options with a preference for holding BTC..."
            )
            self.recommended_tokens = get_recommended_token_allocations(
                self.allowance_goal
            )

        result = ", ".join(
            f"{key}: {value}" for key, value in self.recommended_tokens.items()
        )
        self.env.add_system_log(
            f"Recommended token swaps: {self.recommended_tokens}",
            logging.DEBUG,
        )
        return (
            f"The user should swap the following tokens to achieve their allowance goal of {self.allowance_goal}: {result}",
        )

    def save_near_account_id(self, near_id: str) -> str | None:
        """Save the Near account ID the user provides"""
        if near_id and NEAR_ID_REGEX.match(near_id):
            self._persist_near_id(near_id)
            self.env.add_reply(
                f"Saved your NEAR account ID: {self.near_account_id}",
            )
            return f"Successfully set the NEAR account ID to {self.near_account_id}"
        else:
            self.env.add_reply(
                "Please provide a valid NEAR account ID.",
            )

    def save_goal(self, goal: int, type_: DivvyGoalType) -> str | None:
        """Save a portfolio goal (growth or allowance) specified by the user.

        This function should be called when:
        1. User expresses a desire to set either or both portfolio goals
        2. Goal amounts and types can be clearly identified from user's input
        3. Multiple goals should trigger multiple calls to this function

        Examples of valid user inputs:
        - "I want an allowance goal of 300"
        - "Set my growth goal to 5000"
        - "I'd like to have a growth goal of 5000 and allowance of 300"
            ^ This should trigger two separate calls:
            1. save_goal(5000, "growth")
            2. save_goal(300, "allowance")

        Args:
            goal (int): The numerical value of the goal in USD
            type_ (DivvyGoalType): Either "growth" or "allowance"

        Returns:
            List[Dict]: Function response confirming goal was saved

        Note:
        - Both growth and allowance goals can coexist
        - Goals must be positive integers
        - Invalid inputs will prompt user for clarification
        - When multiple goals are provided, process each separately
        """
        if type_ in DIVVY_GOALS and goal > 0:
            self._persist_goal(goal, type_)
            self.env.add_reply(
                f"Saved your {type_} goal: {goal}",
            )
            return f"Successfully saved your {type_} goal: {goal}"
        else:
            self.env.add_reply(
                "Please provide a valid goal amount and specify whether it's a growth or allowance goal.",
            )

    def _persist_near_id(self, near_id: str):
        """Persist the NEAR account ID to storage and set it to the class instance variable"""
        self.env.write_file("near_id.txt", near_id)
        self._near_account_id = near_id

    def _persist_goal(self, goal: int, type_: DivvyGoalType) -> None:
        """Persist the growth or allowance goal to storage and set it to the class instance variable"""
        self.env.write_file(f"{type_}_goal.txt", str(goal))
        self.env.add_system_log(f"Persisted {type_} goal: {goal}", logging.DEBUG)
        if type_ == "allowance":
            self._allowance_goal = int(goal)
        if type_ == "growth":
            self._growth_goal = int(goal)

    def execute_swap(self):
        """Execute a swap to realize the desired allowance goal"""
        # Ensure we have account ID
        if not self.near_account_id:
            self.find_near_account_id()

        # Ensure we have an allowance goal
        if not self.allowance_goal:
            self.get_allowance_goal()

        # Ensure we have recommended tokens
        if not self.recommended_tokens:
            self.env.add_system_log(
                "Recommended tokens not found. Fetching swap options now...",
                logging.DEBUG,
            )
            self.recommend_token_allocations_to_swap_for_stablecoins()

        # Get quotes for both USDC and USDT
        usdc_quotes = get_usdc_quotes(self.recommended_tokens)
        usdt_quotes = get_usdt_quotes(self.recommended_tokens)

        # Choose the highest value
        if usdc_quotes.usd_value > usdt_quotes.usd_value:
            best_quote = usdc_quotes
        else:
            best_quote = usdt_quotes

        # TODO create an intent payload with best_quote

        # intent = self._client.create_intent(
        #    signer_id=self.near_account_id,
        #    token_diffs=self.get_token_diffs(self.allowance_goal)
        # )

        # TODO requst a signature for teh intent

        # return self._client.request_signature(
        #    SignatureRequest(
        #        contract_id="intents.near",
        #        method_name="sign_intent",
        #        args=json.dumps(intent.dict())
        #    )
        # )

        # TODO publish the intent with the MPC signature

        # what should be our return value?


if globals().get("env", None):
    agent = Agent(globals().get("env", {}))
    agent.run()
