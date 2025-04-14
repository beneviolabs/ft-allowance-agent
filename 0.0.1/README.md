### setup

1. Create a wallet if you don't have one via [Bitte](https://wallet.bitte.ai)

1. Sign up on [near.ai](https://app.near.ai/) with your Bitte wallet.

1. Install nearai CLI

   ```sh
   pip install nearai
   nearai version
   ```

1. Log the CLI in and auth with your Bitte wallet

   ```sh
   nearai login
   ```

1. Create a directory named after your Near account in the `~/.nearai/registry` directory

   ```sh
   mkdir -p ~/.nearai/registry/my-acc.near
   ```

1. Clone the repo and install the dependencies

   ```sh
   cd ~/.nearai/registry/my-acc.near
   git clone git@github.com:beneviolabs/ft-allowance-agent.git
   cd ft-allowance-agent
   python3 -m venv .venv && . .venv/bin/activate
   pip install -r requirements.txt
   ```

1. Install NEAR CLI via this [link](https://docs.near.org/tools/near-cli) and connect our test agent account

```
near account import-account benevio-labs.testnet
```

1. Generate a private key and set the following in your env vars

```
export AGENT_ACCOUNT_ID="benevio-labs.testnet"
export AGENT_SECRET_KEY="<secret-key-from-previous-step>"
```

1. Run the agent locally

   ```sh
   nearai agent interactive ~/.nearai/registry/<your-acc>.near/ft-allowance/0.0.1 --local
   # > hi
   # < Assistant: Hello! I'm here
   ```


1. Follow SDK logs in a different terminal

   ```sh
   tail -n 20 -f /tmp/nearai-agent-runner/ptke.near/ft-allowance-agent/0.0.1/system_log.txt
   ```


## Troubleshooting

### The bot doesn't respond
- Check if the agent file code is running. Add the logging line mentioned below if you don't see any errors.
   > ℹ️ In `nearai/agents/agent.py` file in your site-packages:

   ```python
      def run_python_code(...):
      ...
      # ln: 152
      except ...:
         print(f"Error running agent code: {e}")
      ```
- Add logs to the nearai library functions like completion_and_get_tools_calls() and completions()

### The bot is hallucinating
- Check the prompt construction. Check how many messages are being passed in context
- Ensure you're not using `llama-3p3-70b-instruct` model. It doesn't seem to work well with nearai and returns responses
as _assistant_ messages instead of tool calls.
- If there was a tool call, check its docstring. Only the first line is added to the tool description, the rest of the lines are parsed as args.
- Consider adding [few-shot examples](https://blog.langchain.dev/few-shot-prompting-to-improve-tool-calling-performance/) to the prompt.


