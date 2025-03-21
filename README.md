#### This repo contain the two products

1. [A proxy account](https://github.com/beneviolabs/ft-allowance-agent/blob/main/0.0.1/src/auth_proxy.rs) to be deployed onto a user's sub-account ([with supporting scripts](https://github.com/beneviolabs/ft-allowance-agent/tree/main/0.0.1/src/scripts)), allowing one to transfer tokens to their sub-account and grant an AI agent (i.e any other Near Account) permission to call this proxy contract to request MPC approval to send transactions to a predefined set of contracts and methods (i.e. near_deposit on wrap.testnet). Thereby allowing an Agentic account to act autonomously on your behalf with restricted permissions and access only to the tokens that you transfer to your sub-account.

2. A WIP [Token Allowance Agent ](https://github.com/beneviolabs/ft-allowance-agent/blob/main/0.0.1/agent.py)that capitalizes on market volatility to grow your wealth, by determining which tokens to periodically swap into stablecoins to secure gains without reducing your portfolio below some minimum USD value. Realize these gains for yourself, or setup a conditional recurring allowance for your crypto curious friends & family.


#### A Proxy Account to allow an agent autonomous, yet restricted use of select assets 
This repo introduces auth_proxy.rs to be deployed onto a user's sub-account (with scripts supporting deployment and teseting), allowing one to transfer tokens to their sub account and grant an AI agent (i.e any other Near Account) permission to call this proxy contract to request MPC approval to send transactions to a predefined set of contracts and methods (i.e. near_deposit on wrap.testnet). Thereby allowing an Agentic account to act autonomously on your behalf with restricted permissions and access only to the tokens that you transfer to your sub-account.

#### Examples
1. [This testnet txn](https://testnet.nearblocks.io/txns/Hi2pfe89tBdMN2oY2dFXLuHcSBVFotx6pHViDQuKUZDi) converting 1 Near to WNear by `agent.charleslavon.testnet` was initiated by `benevio-labs.testnet` who was pre-approved by auth_proxy.rs to use Near's MPC contract to [create a signature](https://testnet.nearblocks.io/txns/831u2KqbdtzvJti5HUhGnp4tZD7Q8onUzD11rwBjrAAm).
2. [This mainnet txn](https://nearblocks.io/txns/GRw6oEWjAQ2QT9oDtsgBSRWr3s4oCW4A8zCpHCRXD62s) adding a public_key onto `intents.near` was initiated by `benevio-labs.near` who was pre-approved by auth_proxy.rs to [create an MPC signature](https://nearblocks.io/txns/9PJXbvcb4RMxjwK8VW4N54RnvrjENUCr6N1nv9f3DZJQ).


#### Setup dependencies
1. Install near-cli-rs
2. Set your target network as an environment variable e.g. `export NEAR_ENV=testnet`
3. Login with a near testnet account and choose to save the private key into your mac's keychain, `near login`
4. Need tokens? Use a [Near testnet faucet](https://near-faucet.io/) to fund your account. 

#### Setup your proxy account 
1. From the terminal, navigate to `/ft-allowance/0.0.1/src/scripts` 
3. Create the `agent.your-account.testnet` sub account to be used by autonomous agents by passing your testnet account as an argument to `./create_sub_account.sh your-account.testnet`
4. Deploy the proxy contract to your sub account: `./deploy.sh`
5. You will need a 2nd testnet account to complete step 5, use `near login` to ensure near-cli has access to another near testnet account of yours.
6. Grant the ability to use the proxy contract to request signatures and transact on your behalf to another near account, another account that you control or an agentic account, `./add_authorized_agent.sh your-other-account.testnet`
7. Use your proxy accounts full access public key to derive a key to be used by the MPC signer, and have that MPC key added onto your proxy account with full access permissions: `./derive_mpc_key.sh` 

#### Test Requesting Signatures
1. Go to [NearBlocks](https://testnet.nearblocks.io/), on the upper right select the `Near Icon`, then `testnet`, then click into a `Latest Block` and copy the block hash.  Now you can simulate a program or agent using your proxy contract by requesting a signature, `./request_signature.sh <block hash> < add_key | deposit > <your-other-account.testnet>`
2. If successful, transaction logs (view them in your terminal or on nearblocks.io) should display the Reconstructed Signature in base64 (scroll up or search for `Signed transaction (base64)`).  Pass this signature `./submit_txn.sh` to test broadcasting this testnet transaction. `./submit_txn.sh FAAAAGNoYXJsZXNsYXZvbi50ZXN0bmV0AQD1k+Pq3bhLFaNXClzgx0fEBmZItkkolypTJq0v0O6JOB856PxW5l+TZwD6MTrEBY+xsI/3wBgz2RNY+Ax5RETZq+2FlQEAAAwAAAB3cmFwLnRlc3RuZXQwRFzWCwWaY4pPFHl46Bj87dj6JLtdm28rjKf37iFc4QEAAAACDAAAAG5lYXJfZGVwb3NpdAAAAAAAoHJOGAkAAAAAAKHtzM4bwtMAAAAAAAAButebmlYXbKcuRM9NfWfgOAdR9jzGvS4Fv53T4/wOGjwwjizI0PvKnpaCpsxkNyTFZHQEVpYkCNPnUbabAYYx/QI=`


#### Notes

##### linting
`autopep8 --in-place --aggressive --aggressive *.py`

##### run the agent locally
`nearai agent interactive ~/.nearai/registry/charleslavon.near/ft-allowance/0.0.1 --local`

##### download a published agent
`nearai registry download zavodil.near/swap-agent/latest`



