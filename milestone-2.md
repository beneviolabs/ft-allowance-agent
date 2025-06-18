#### Milestone 2

<b>Audience:</b> Mainstream Consumers.
<b>Overview:</b> Your self-described luddite friend finds Peerlio from a web search, after a brief chat with the Peerfolio Agent, Friend is convinced that they will use Peerfolio as their first venture into acquiring crypto.


- when do onboarding preferences come into play?

#### Major enhancements: // TODO finish user journeys for each of these
- Product Design for multiplayer mode
- auto execute conditional swap goals given market conditions
- adds SMS notifications
- design the flow of supporting a all of the below in input/output token types. Must I generate a chainsiggy address for the corresponding token type? Or can they stay on the intent address associated with the near trading account? The latter should be possible via one-click's [recipientType=INTENTS](https://docs.near-intents.org/near-intents/integration/distribution-channels/1click-api#post-v0-quote)

how to query intents to see assets asscoatied to a near account within intents?
 by a view function call for each token, to [intents.near](https://docs.near-intents.org/near-intents/market-makers/verifier/deposits-and-withdrawals/balances-and-identifying-your-token#checking-your-balance)

- Support ETH, SOL, XRP, and BTC in the deposit to token aquisition flow
- adds withdraw all / close your trading account.
- support withdrawal of a specific token and amount from trading account
  how to use one-click to withdraw from intents to destination chain? - one must near rpc call the 'withdraw' method on each token to be withdrawn, then use intent's PoA's bridge API [query the withdrawl status](https://docs.near-intents.org/near-intents/market-makers/poa-bridge-api#how-to-use)

- technical design support for mainstream consumers:
  - via a passkey-based onboarding flow. create a flow diagram to vet the use of webAuthn and fully plan this architecture.


##### SMS Notifications:
1. During onboarding: We'll need to ask for the user's phone number, and verify it by sending and requesting they enter a OTP.
1. On successful automated swap: "Cha-ching. Your $ goal was realized. You now have an extra [USD-amount] in your trading account."
1. From milestone-1, User declines to set a swap goal due to current market state:
  Agent says: Totally understood — I won’t suggest any goals for now. Would you like me to ping you when your portfolio crosses back above $200? I can send a message via SMS or Telegram — just let me know.

  User: yes

  Agent: Ok. I'll text you if/when your portfolio crosses above $200 and we can discuss creating an allowance goal.

  On portfolio crossing USD threshold: "Your portfolio/peerfolio? has reached new heights. Let's review the details and consdier new goals <app link>"

1. Do we need to support any additional notifications at this stage?


#### Auto execute conditional swap goals given market conditions



##### Support ETH, SOL, XRP, and BTC in the deposit to token aquisition flow
1. Store in the client, an initial allocation object where Key=token_ticker Value=percentage_allocation. USDC should be included here.
1. The architecture will be expanded upon in milestone 3 as themed asset allocation buckets that will be used to guide users towards converting their deposits into these sets of tokens.
1. Allow the agent to take action when a user makes a deposit, to initiate the purchase of all of the assets in the initial allocation bucket.
1.


##### Withdrawal Some: User wants to withdraw select tokens to their external wallets. - should this be agent based or fully UI based?
1. Alice: Hey, I want to move tokens out of my trading account.
1. Agent: Ok. Would you like to to move all the tokens from your trading account to your main account?
1. Alice: "No I only want to move 3 Near and 10 ETH" or "No I want to withdraw $200 worth."
1.



##### Withdrawal All: User wants to withdraw all / close trading account
1. Alice: Hey, I want to move tokens out of my trading account.
1. Agent: Ok. Would you like to to move all the tokens from your trading account to your main account?
1. Alice: Yes
1. Agent: You have a total of 10 Near, 1 ETH, and .5 SOL in your trading account. If you plan to continue using Peerfolio, we need to keep a minimum of 5 Near on your trading account to keep it active. Do you plan to continue to use Peerfolio?
1. Alice: No
1. Agent: In a few seconds, you will be asked to review and approve a transaction to move X Near to alice.near.
User interacts with the wallet and then returns to Peerfolio.
1. Agent: ✅ I’ve moved your tokens back to your main account. You can view the transaction here if you’d like. Let me know if you want to create a new goal or check your portfolio details.



##### Passkey-based onboarding flow for Normies

