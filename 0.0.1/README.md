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

1. Run the agent locally

   ```sh
   nearai agent interactive ~/.nearai/registry/<your-acc>.near/ft-allowance/0.0.1 --local
   # > hi
   # < Assistant: Hello! I'm here
   ```

   > ℹ️ In case you don't get any responses, add print to the library `nearai/agents/agent.py` file in your site-packages

   ```python
   def run_python_code(...):
     ...
     # ln: 152
     except ...:
       print(f"Error running agent code: {e}")
   ```

   Other SDK logs can be found at `/tmp/nearai-agent-runner/ptke.near/ft-allowance-agent/0.0.1/system_log.txt`.

## Deploying

1. Ensure you have an empty app called `test-app` in your Near account. If not, use nearai CLI to create and deploy it.

1. Copy `metadata.dev.json` into `metadata.json` and ensure `show_entry` is set to true then run the following from the 0.0.1 directory
   ```
   nearai registry upload --bump
   ```

   > ⚠️‼️ Caution: Every file in the 0.0.1 directory will be uploaded to the app, so make sure you don't have any sensitive files in there.

1. Go to your nearai dashboard and find your new deployment

1. If you wanna hide the bot, set the show_entry field in metadata.json file to _false_ and run `nearai registry upload --bump` again to set the listing to private.

1. If there is a dependency that needs to be updated, we need to submit a pull request to the nearai repo to update their aws_runner dependencies over [here](https://github.com/nearai/nearai/blob/main/aws_runner/frameworks/requirements-standard.txt).

### Setting secrets

- You can set secrets in the nearai dashboard. They will be available in the env vars of the running agent. When set there, they will be considered agent secrets and will be available to anyone running your agent.

   So for example if you are user exampleuser.near with agent foobar, any secret you set for foobar will be usable but not visible to otheruser2.near who uses your agent.


### Troubleshooting deployments and the deployed running application

- Add `DEBUG=true` to the env vars and check the `ℹShow system logs` checkbox that sits left to the send message button. This will show you stacktraces and debug level logs.

- Deploy doesn't show up: If a newly deployed version doesn't show up on the UI check the metadata.json. if you got any fields wrong or missing your new version might not be listed. Also make sure `show_entry` is set to true.

- If the description field is too long you will get a HTTP 500 error.
