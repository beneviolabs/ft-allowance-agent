import logging
import inspect
import json
import re
from src.client import NearMpcClient
from src.models import SignatureRequest, MpcKey, Intent
import typing
from nearai.agents.environment import Environment, ChatCompletionMessageToolCall
from src.utils import (
    fetch_coinbase,
    fetch_coingecko,
    get_recommended_token_allocations,
    get_near_account_balance,
    yocto_to_near,
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
        self._client = NearMpcClient(network=env.env_vars["network"])
        tool_registry = self.env.get_tool_registry()
        tool_registry.register_tool(
            self.recommend_token_swaps
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
    def allowance_goal(self) -> int | None:
        if self._allowance_goal is None:
            stored_goal = self.env.read_file("allowance_goal.txt")
            self.env.add_system_log(
                f"Found stored allowance goal: {stored_goal}", logging.DEBUG)
            if stored_goal:
                self._allowance_goal = int(stored_goal)
        return self._allowance_goal

    @property
    def growth_goal(self) -> int | None:
        if self._growth_goal is None:
            stored_goal = self.env.read_file("growth_goal.txt")
            self.env.add_system_log(
                f"Found stored growth goal: {stored_goal}", logging.DEBUG)
            if stored_goal:
                self._growth_goal = int(stored_goal)
        return self._growth_goal

    def run(self):
        # A system message guides an agent to solve specific tasks.
        prompt = {
            "role": "system",
            # disallow some builtin tools from the library
            "content": """
You are Divvy, a financial assistant that helps users manage and grow their crypto portfolio.
You can access NEAR account details of the user (their balance and account id).
You can provide the real-time current market prices of crypto tokens in the users wallet.
You can allow the user to set growth and allowance goals on their portfolio.
You are capable of recommending token swaps to achieve the user's allowance goal in stablecoins.
You can also execute the token swaps to realize the desired allowance goal.
You can also fetch the NEAR account balance of the user.

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
            tool_call_results = self._handle_tool_calls(
                tools_completion.tool_calls)
            if len(tool_call_results) > 0:
                self.env.add_system_log(
                    f"Got tool call results: {tool_call_results}", logging.DEBUG
                )

                context = [prompt] + self.env.list_messages() + \
                    tool_call_results
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

        responses.append(self._to_function_response(
            tool_name, self.near_account_id))
        return responses

    def get_near_account_balance(self) -> typing.List[typing.Dict]:
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

        tool_name = self._get_tool_name()
        responses = []
        balance = None
        if self.near_account_id:
            balance = get_near_account_balance(self.near_account_id)
            if balance:
                self.near_account_balance = yocto_to_near(balance)
            self.env.add_reply("Found the user's balance",
                               message_type="system")
        else:
            self.env.add_reply(
                "We couldn't fetch a balance because no NEAR account ID is set. What is your near account ID?",
                message_type="system",
            )
        responses.append(self._to_function_response(tool_name, balance))
        return responses

    # IMPROVE: this function can be parameterized to only query prices for tokens user specifies and fetch all if there's no param value
    def fetch_token_prices(self):
        """Fetch the real-time market prices of the tokens in a user's wallet (e.g. NEAR, BTC, ETH, SOL)"""
        tool_name = self._get_tool_name()

        self.env.add_reply(
            "Fetching the current prices of the tokens in your wallet..."
        )
        near_price = fetch_coinbase("near")
        near_price = (
            fetch_coingecko("near") if isinstance(
                near_price, bool) else near_price
        )

        btc_price = fetch_coinbase("btc")
        btc_price = fetch_coingecko("btc") if isinstance(
            btc_price, bool) else btc_price

        eth_price = fetch_coinbase("eth")
        eth_price = fetch_coingecko("eth") if isinstance(
            eth_price, bool) else eth_price

        sol_price = fetch_coinbase("sol")
        sol_price = fetch_coingecko("sol") if isinstance(
            sol_price, bool) else sol_price
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

        responses.append(self._to_function_response(
            self._get_tool_name(), goals))
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

    def recommend_token_swaps(self) -> typing.List[typing.Dict]:
        """
            Generate token swap recommendations to achieve the user's allowance goal in stablecoins.

            This function should be called when:
            1. The user has set an allowance goal
            2. The user requests advice on which tokens to swap

            The function will:
            1. Consider the user's portfolio composition
            2. Prioritize maximizing BTC holdings
            3. Recommend optimal token quantities to swap into USDC/USDT
            4. Preserve long-term growth potential while meeting allowance needs

            Returns:
                List[Dict]: A list containing a single function response with recommended swaps:
                [{
                    'token': str,           # Token symbol (e.g., 'NEAR', 'SOL')
                    'amount': float,        # Amount of tokens to swap
                }]

            Example response:
                [{'role': 'function',
                'name': 'recommend_token_swaps',
                'content': '{'NEAR': 99.04298327119977, 'SOL': 1.0781411622775472, 'ETH': 0.07348127071321557, 'BTC': 0.02280692838780696}'
                }]
        """

        tool_name = self._get_tool_name()

        # Log allowance goals
        self.env.add_system_log(
            f"Current allowance goal: {self.allowance_goal}",
            logging.DEBUG
        )

        if self.allowance_goal is None:
            goals = self.get_goals()[0]
            goals_dict = json.loads(goals["content"])
            if goals_dict["allowance"]:
                self.save_goal(int(goals_dict["allowance"]), "allowance")

        if not self.recommended_tokens:
            self.env.add_reply(
                f"Considering your options with a preference for holding BTC..."
            )
            self.recommended_tokens = get_recommended_token_allocations(
                int(self.allowance_goal)
            )

        if self.recommended_tokens:
            self.env.add_system_log(
                f"Recommended token swaps: {json.dumps(self.recommended_tokens)}",
                logging.DEBUG
            )
        return [self._to_function_response(tool_name, self.recommended_tokens or [])]

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

    def _persist_goal(self, goal: int, type_: DivvyGoalType) -> None:
        """Persist the growth or allowance goal to storage and set it to the class instance variable"""
        self.env.write_file(f"{type_}_goal.txt", str(goal))
        if type_ == "allowance":
            self.env.add_system_log(
                f"Persisted {type_} goal: {goal}",
                logging.DEBUG
            )
            self._allowance_goal = goal
        if type_ == "growth":
            self._growth_goal = goal

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
                f"Recommended tokens not found. Fetching swap options now...",
                logging.DEBUG
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


if globals().get('env', None):
    agent = Agent(globals().get('env', {}))
    agent.run()
