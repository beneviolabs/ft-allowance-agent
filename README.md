### Onboarding sequence

These are all the various components that interact during a user's onboarding.

  ```mermaid
sequenceDiagram
    autonumber
    participant Wallet as NEAR (& Wallet) <br> (crypto-native wallet<br>user.near)
    participant User as User <br> (Browser)
    participant ProxyFac as Proxy Factory <br>(ProxyFactory contract<br>auth-v0.peerfolio.near)
    participant TradingAcc as Proxy/Trading Account <br>(user.auth-v0.peerfolio.near)
    participant MPC as MPC Contract

    User->>Wallet: Connect wallet
    Wallet->>User: Function call key for <br> auth-v0.peerfolio.near <br> (limited access)
    critical Approve txn
        User->>Wallet: (deposit_and_create_proxy) <br> w/ 4 Ⓝ
    option no balance
        Wallet--xUser: TBD
    option timeout/browser window closed
        Wallet-->User: No effect
    end
    critical deposit_and_create_proxy()
        Wallet->>ProxyFac: deposit_and_create_proxy()
        ProxyFac->>TradingAcc: i. create proxy account<br>ii. transfer deposit<br>iii.deploy AuthProxy contract<br>iv. call AuthProxy.new to<br> set authorized user (user.near) <br> and MPC signer (v1.signer))
    option trading acc already exists

    option other error
        ProxyFac--xWallet: Refund 4 Ⓝ
    end

    critical MPC key registration
        User->>MPC: derive MPC public key for trading account
    option service unavailable
        MPC--xUser: Retry
        User->>Wallet: Approve add MPC key + <br> authorized user (peerfolio.near) txn
        Wallet->>TradingAcc: MPC key with full access is set
    end
  ```

Examples
  - Approve txn: https://testnet.nearblocks.io/txns/CF1ainGjroxtppNTWWkFgQsiC5kC4iJ3X7v8FgLMrWDE?tab=execution
  - deposit and create proxy: https://testnet.nearblocks.io/txns/CF1ainGjroxtppNTWWkFgQsiC5kC4iJ3X7v8FgLMrWDE?tab=execution#GV1aG4CqY6Lm2L28yb3tdinD2vZ7mZRS8fvPfBzahcnp
  - MPC key registration: https://testnet.nearblocks.io/txns/5Jyn459DhAaRxEqvTo3x724cgxCpjTH1Jaoc7uyVNQt9

### Agent execution sequence (swaps etc.)

TBD

#### This repo contain the two products

1. [The limited access trading account](https://github.com/beneviolabs/ft-allowance-agent/blob/main/contracts/auth_proxy.rs) manages authorized users for signature requests, allows one to transfer tokens to their trading and grant an AI agent (i.e any other Near Account) permission to call this proxy contract to request MPC approval to send transactions to a predefined set of contracts and methods (i.e. near_deposit on wrap.testnet). Thereby allowing an Agentic account to act autonomously on your behalf with restricted permissions and access only to the tokens that you transfer to your trading account. The system consists of two main contracts:
    1. Factory Contract (factory.rs):

    - Acts as a proxy contract deployer
    - Stores proxy code hash for verification
    - [Creates proxy instances](https://testnet.nearblocks.io/txns/8Q8mPTCUxaJnTubwE6ZTHF1ZLvxo9BhVfBBifsuriXAD) with proper initialization
    - Ensures secure deployment with minimum deposit requirements
    Example usage: `near call auth-factory.benevio-labs.testnet create_proxy \
  '{"owner_id": "alice.testnet"}' \
  --accountId benevio-labs.testnet \
  --deposit 5`

    2. Auth Proxy Contract (auth_proxy.rs):

    - Manages authorized users for signature requests
    - Handles MPC signature generation for approved transactions
    - Restricts contract interactions to predefined set (wrap.near, intents.near)
    - Supports specific methods (near_deposit, add_public_key, etc.)
    Example usage: `near call alice.auth-factory.benevio-labs.testnet request_signature \
  '{...signature_args...}' \
  --accountId authorized-agent.testnet`

2. A WIP [Token Allowance Agent ](https://github.com/beneviolabs/ft-allowance-agent/blob/main/0.0.1/agent.py)that capitalizes on market volatility to grow your wealth, by determining which tokens to periodically swap into stablecoins to secure gains without reducing your portfolio below some minimum USD value. Realize these gains for yourself, or setup a conditional recurring allowance for your crypto curious friends & family.

### Deleting a trading/proxy account

1. Add your main account public key to the proxy account with full access permissions
```
near contract call-function as-transaction <mainaccount>.auth-v0.peerfolio.testnet add_full_access_key json-args '{"public_key": "<main-account-public-key>"}' prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' sign-as <mainaccount>.testnet network-config testnet sign-with-keychain send
```

2. Send a delete account transaction signing with your main account key pair
```
near account delete-account <mainaccount>.auth-v0.peerfolio.testnet beneficiary <mainaccount>.testnet network-config testnet sign-with-plaintext-private-key --signer-public-key <main-account-public-key> --signer-private-key <main-account-private-key> send
```

#### TODO - Replace the examples and scripts with details on how to onboard via the langchain agent.

#### Examples
1. [This testnet txn](https://testnet.nearblocks.io/txns/Hi2pfe89tBdMN2oY2dFXLuHcSBVFotx6pHViDQuKUZDi) converting 1 Near to WNear by `agent.charleslavon.testnet` was initiated by `benevio-labs.testnet` who was pre-approved by auth_proxy.rs to use Near's MPC contract to [create a signature](https://testnet.nearblocks.io/txns/831u2KqbdtzvJti5HUhGnp4tZD7Q8onUzD11rwBjrAAm).
2. [This mainnet txn](https://nearblocks.io/txns/GRw6oEWjAQ2QT9oDtsgBSRWr3s4oCW4A8zCpHCRXD62s) adding a public_key onto `intents.near` was initiated by `benevio-labs.near` who was pre-approved by auth_proxy.rs to [create an MPC signature](https://nearblocks.io/txns/9PJXbvcb4RMxjwK8VW4N54RnvrjENUCr6N1nv9f3DZJQ).


#### Setup dependencies
1. Install near-cli-rs
2. Set your target network as an environment variable e.g. `export NEAR_ENV=testnet`
3. Login with a near testnet account and choose to save the private key into your mac's keychain, `near login`
4. Need tokens? Use a [Near testnet faucet](https://near-faucet.io/) to fund your account.
5. Build and install rust tooling

    ```bash
    # if running on Apple Silicon.
    rustup toolchain install nightly-aarch64-apple-darwin
    rustup component add rust-src --toolchain nightly-aarch64-apple-darwin
    scripts/build.sh
    ```

#### Setup your proxy account
1. From the terminal, navigate to `/ft-allowance/contracts/scripts`
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



