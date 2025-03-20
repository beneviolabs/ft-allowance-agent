import logging
import inspect
import json
import re
import typing
from nearai.agents.environment import Environment, ChatCompletionMessageToolCall
from src.utils import (
    fetch_coinbase,
    fetch_coingecko,
    get_recommended_token_allocations,
    get_near_account_balance,
)

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
        tool_registry = self.env.get_tool_registry()
        tool_registry.register_tool(
            self.recommend_token_allocations_to_swap_for_stablecoins
        )
        tool_registry.register_tool(self.get_near_account_id)
        tool_registry.register_tool(self.save_near_account_id)
        tool_registry.register_tool(self.get_goals)
        tool_registry.register_tool(self.save_goal)
        tool_registry.register_tool(self.get_near_account_balance)
        tool_registry.register_tool(self.fetch_token_prices)

    @property
    def near_account_id(self) -> str | None:
        if not self._near_account_id:
            self._near_account_id = self.env.read_file("near_id.txt")
        return self._near_account_id

    @property
    def allowance_goal(self) -> str | None:
        if not self._allowance_goal:
            self._allowance_goal = self.env.read_file("allowance_goal.txt")
        return self._allowance_goal

    @property
    def growth_goal(self) -> str | None:
        if not self._growth_goal:
            self._growth_goal = self.env.read_file("growth_goal.txt")
        return self._growth_goal

    def run(self):
        # A system message guides an agent to solve specific tasks.
        prompt = {
            "role": "system",
            "content": """
You are Divvy, a financial assistant that helps users manage and grow their crypto portfolio.
You have access NEAR account details of a user (such as the balance and id).
You can fetch the current market prices of crypto tokens in a user's wallet.
You can allow the user to set growth and allowance goals on their portfolio.

Your capabilities are defined and facilitated by the tools you have access to except your disallowed tools.

-Disallowed tools-
`list_files, query_vector_store, write_file, request_user_input, read_file, exec_command`.

You must follow the following instructions:

-Instructions-
* Be polite and helpful to the user.
* When introducing yourself, provide a brief description of what your purpose is.
* Tell the user if you don't support a capability. Do NOT make up or provide false information or figures.
* Do not expose the functions you have access to to the user.
* Do not use figures or function call from preceding messages to generate responses.
""",
        }

        # Use the model set in the metadata to generate a response
        result = None
        tools = self.env.get_tool_registry().get_all_tool_definitions()
        tools_completion = self.env.completion_and_get_tools_calls(
            [prompt] + [self.env.get_last_message() or ""], tools=tools
        )
        self.env.add_system_log(
            f"Should call tools: {tools_completion.tool_calls}", logging.DEBUG
        )

        if tools_completion.message:
            self.env.add_reply(tools_completion.message)
        if tools_completion.tool_calls and len(tools_completion.tool_calls) > 0:
            tool_call_results = self._handle_tool_calls(tools_completion.tool_calls)
            if len(tool_call_results) > 0:
                self.env.add_system_log(
                    f"Got tool call results: {tool_call_results}", logging.DEBUG
                )

                context = [prompt] + self.env.list_messages() + tool_call_results
                result = self.env.completion(context)

                self.env.add_system_log(
                    f"Got completion for tool call with results: {result}. Context: {context}",
                    logging.DEBUG,
                )

        if result:
            self.env.add_reply(result)

        # Give the prompt back to the user
        self.env.request_user_input()

    @staticmethod
    def _to_function_response(function_name: str, value: typing.Any) -> typing.Dict:
        """
        Use to tell the LLM the result from a function call in a structured way
        """
        return {
            "role": "function",
            "name": function_name,
            "content": json.dumps(value),
        }

    def _get_tool_name(self) -> str:
        """Return the function name of the calling function as the tool name"""
        return inspect.stack()[1][3]

    def get_near_account_id(self) -> typing.List[typing.Dict]:
        """Get the NEAR account ID of the user"""
        tool_name = self._get_tool_name()
        responses = []
        if not self.near_account_id:
            self.env.add_reply(
                "There is no NEAR account ID right now. Please provide one.",
                message_type="system",
            )

        responses.append(self._to_function_response(tool_name, self.near_account_id))
        return responses

    def get_near_account_balance(self) -> typing.List[typing.Dict]:
        """Get the NEAR account balance of the user"""
        tool_name = self._get_tool_name()
        balance = get_near_account_balance(self.near_account_id)
        return [self._to_function_response(tool_name, balance)]

    # IMPROVE: this function can be parameterized to only query prices for tokens user specifies and fetch all if there's no param value
    def fetch_token_prices(self):
        """Fetch the current market prices of the tokens in a user's wallet (e.g. NEAR, BTC, ETH, SOL)"""
        tool_name = self._get_tool_name()
        balance = get_near_account_balance(self.near_account_id)
        if balance:
            if len(balance) > 23:
                length = len(balance)
                chars_remaining = length - 23
                # TODO improve the yocoto to Near conversion
                self.near_account_balance = float(
                    str(balance[0 : chars_remaining - 1])
                    + "."
                    + "".join(balance[chars_remaining - 1 : length])
                )

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
        self.prices = [
            "NEAR:",
            near_price,
            "BTC:",
            btc_price,
            "ETH:",
            eth_price,
            "SOL:",
            sol_price,
        ]

        return [self._to_function_response(tool_name, self.prices)]

    def get_goals(self):
        """
        Return the goals that the user has set for their portfolio
        This includes growth and allowance goals
        """
        responses = []
        goals = {}
        for type_, goal in [
            ("growth", self.growth_goal),
            ("allowance", self.allowance_goal),
        ]:
            if not goal:
                self.env.add_reply(
                    f"The user hasn't set a {type_} goal yet. Prompt them to provide one.",
                    message_type="system",
                )
            goals[type_] = goal

        responses.append(self._to_function_response(self._get_tool_name(), goals))
        return responses

    def _handle_tool_calls(
        self, tool_calls: typing.List[ChatCompletionMessageToolCall]
    ) -> typing.List[typing.Dict]:
        """Execute the tool calls and return a result for the LLM to process"""
        results = []
        for tool_call in tool_calls:
            # exec_command tool call seems to be for executing commands on the Terminal? probably should be deregistered
            if tool_call.function.name == "exec_command":
                continue
            tool = self.env.get_tool_registry().get_tool(tool_call.function.name)
            if not tool:
                self.env.add_system_log(
                    f"Tool '{tool_call.function.name}' not found in the tool registry.",
                    logging.WARNING,
                )
                continue
            args = json.loads(tool_call.function.arguments)
            results.extend(tool(**args))
        return results

    def recommend_token_allocations_to_swap_for_stablecoins(self):
        """Given a input of a target USD amount, recommend the tokens and quantities of each to swap for USDT stablecoins or USDC stablecoins"""
        if not self.recommended_tokens:
            self.env.add_reply(
                f"Considering your options with a preference for holding BTC..."
            )
            self.recommended_tokens = get_recommended_token_allocations(
                int(self.allowance_goal)
            )

        self.env.add_reply(
            f"We can sell this quantity of your tokens to realize your target USD in stablecoin..."
        )
        return str(self.recommended_tokens) if self.recommended_tokens else ""

    def save_near_account_id(self, near_id: str) -> typing.List[typing.Dict]:
        """Save the Near account ID the user provides"""
        responses = []
        if near_id and NEAR_ID_REGEX.match(near_id):
            self._persist_near_id(near_id)
            self.env.add_reply(
                f"Saved your NEAR account ID: {self.near_account_id}",
            )
        else:
            self.env.add_reply(
                "Please provide a valid NEAR account ID.",
            )
        return responses

    def save_goal(self, goal: int, type_: DivvyGoalType) -> typing.List[typing.Dict]:
        """Save the growth or allowance goal the user provides.
        If they don't specify which goal type, ask them to disambiguate.
        """
        responses = []
        if type_ in DIVVY_GOALS and goal > 0:
            self._persist_goal(goal, type_)
            self.env.add_reply(
                f"Saved your {type_} goal: {goal}",
            )
        else:
            self.env.add_reply(
                "Please provide a valid goal amount and specify whether it's a growth or allowance goal.",
            )
        return responses

    def _persist_near_id(self, near_id: str):
        """Persist the NEAR account ID to storage and set it to the class instance variable"""
        self.env.write_file("near_id.txt", near_id)
        self._near_account_id = near_id

    def _persist_goal(self, goal: int, type_: DivvyGoalType) -> int:
        """Persist the growth or allowance goal to storage and set it to the class instance variable"""
        self.env.write_file(f"{type_}_goal.txt", str(goal))
        if type_ == "allowance":
            self._allowance_goal = goal
        if type == "growth":
            self._growth_goal = goal


if globals().get("env", None):
    env = globals().get("env", {})
    agent = Agent(env)
    agent.run()
