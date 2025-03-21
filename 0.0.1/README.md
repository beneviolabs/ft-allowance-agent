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

1. Run the agent locally

   ```sh
   nearai agent interactive ~/.nearai/registry/<your-acc>.near/ft-allowance-agent/0.0.1 --local
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
