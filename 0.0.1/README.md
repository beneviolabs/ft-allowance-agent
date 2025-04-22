## setup

This is the Python source code.

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
   tail -n 20 -f /tmp/nearai-agent-runner/<your-acc>.near/ft-allowance-agent/0.0.1/system_log.txt
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
- Prompt and context: Check the prompt construction. Check how many messages are being passed in the context and whether anything in them could be inducing hallucinations.
- Model: Ensure you're not using `llama-3p3-70b-instruct` model. It doesn't seem to work well with nearai and returns responses
as _assistant_ messages instead of tool calls.
- Tools: If there was a tool call, check its docstring since this also affects the prompt. Only the first line is added to the tool description, the rest of the lines are parsed into arg parameters and descriptions e.g. `{'type': 'function', 'function': {'name': 'save_goal', 'description': 'Save a portfolio goal (growth or allowance) specified by the user.', 'parameters': {'type': 'object', 'properties': {'goal': {'description': 'The numerical value of the goal in USD', 'type': 'integer'}, 'type_': {'description': 'Either "growth" or "allowance"', 'type': 'string'}}, 'required': ['goal', 'type_']}}}`.
- Examples: Consider adding [few-shot examples](https://blog.langchain.dev/few-shot-prompting-to-improve-tool-calling-performance/) to the prompt to help guide the bot in generating a response.
These live in fewshots.py.

## Deploying

1. Ensure you have an empty app called `test-app` in your Near account. If not, use nearai CLI to create and deploy it.

1. Copy `metadata.dev.json` into `metadata.json` and ensure `show_entry` is set to true then run the following from the 0.0.1 directory
   ```
   nearai registry upload --bump
   ```

   > ⚠️‼️ Caution: Every file in the 0.0.1 directory will be uploaded to the app, so make sure you don't have any sensitive files in there.

1. Go to your nearai dashboard and find your new deployment

1. If you wanna hide the agent you deployed, set the show_entry field in metadata.json file to _false_ and run `nearai registry upload --bump` again to set the listing to private this way it won't show up in the dashboard search. It will still be accessible if you have a link to it.

1. If there is a dependency that needs to be updated, we need to submit a pull request to the nearai repo to update their aws_runner dependencies over [here](https://github.com/nearai/nearai/blob/main/aws_runner/frameworks/requirements-standard.txt).

### Setting secrets

- You can set secrets in the nearai dashboard. They will be available in the env vars of the running agent. When set there, they will be considered agent secrets and will be usable but not visible by anyone running your agent.

   So for example if you are user exampleuser.near with agent foobar, any secret you set for foobar will be present (but not visible) when otheruser2.near uses your agent.

   > ⚠️ Any changes to the secrets you make will also be effected on the other user's runtime. So be careful when changing them.

### Troubleshooting deployments and the deployed running application

- Add `DEBUG=true` to the env vars and check the `ℹShow system logs` checkbox that sits left to the send message button. This will show you stacktraces and debug level logs.

- Deploy doesn't show up: If a newly deployed version doesn't show up on the UI check the metadata.json. if you got any fields wrong or missing your new version might not be listed. Also make sure `show_entry` is set to true.

- If the description field is too long you will get a HTTP 500 error. Special chars also cause this.
