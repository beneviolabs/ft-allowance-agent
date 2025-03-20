### Token Allowance Agent

This repo contain the two products

1. A proxy account to be deployed onto a user's sub-account (with supporting scripts), allowing one to transfer tokens to their sub-account and grant an AI agent (i.e any other Near Account) permission to call this proxy contract to request MPC approval to send transactions to a predefined set of contracts and methods (i.e. near_deposit on wrap.testnet). Thereby allowing an Agentic account to act autonomously on your behalf with restricted permissions and access only to the tokens that you transfer to your sub-account.

2. A WIP Token Allowance Agent that capitalizes on market volatility to grow your wealth, by determining which tokens to periodically swap into stablecoins to secure gains without reducing your portfolio below some minimum USD value. Realize these gains for yourself, or setup a conditional recurring allowance for your crypto curious friends & family.




#### Notes

### linting
`autopep8 --in-place --aggressive --aggressive *.py`

### run the agent locally
`nearai agent interactive ~/.nearai/registry/charleslavon.near/ft-allowance/0.0.1 --local`

### download a published agent
`nearai registry download zavodil.near/swap-agent/latest`



