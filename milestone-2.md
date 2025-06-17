#### Milestone 2

<b>Audience:</b> Mainstream Consumers.
<b>Overview:</b> Your self-described luddite friend finds Peerlio from a web search, after a brief chat with the Peerfolio Agent, Friend is convinced that they will use Peerfolio as their first venture into acquiring crypto.


- when do onboarding preferences come into play?

#### Major enhancements: // TODO finish user journeys for each of these
- auto execute conditional swap goals given market conditions
- adds SMS notifications
- support ETH, SOL, XRP, and BTC in the fund trading account, deposit, and goal swap flows
- adds withdraw all / close your trading account.
- suport withdrawal of a specific USD amount from trading account
- to support mainstream consumers:
  - adds a passkey-based onboarding flow. create a flow diagram to vet the use of webAuthn and fully plan this architecture.


##### Notifications:
1. During onboarding: We'll need to ask for the user's phone number, and verify it by sending and requesting they enter a OTP.
1. On successful automated swap: "Cha-ching. Your USD was realized. You now have an extra [USD-amount] in your trading account."
1. From milestone-1, User declines to act due to current market state:
  Agent says: Totally understood — I won’t suggest any swaps for now. Would you like me to ping you when your portfolio crosses back above $200? I can send a message via SMS or Telegram — just let me know.

  Do we need to support the above scenario right now?

1.

##### User Journey Post Successful Swaps
Context: Ok so now I have USDC on my agent trading account. Now what? How do we faciliate one sending this to their exchange account, e.g. Coinbase.
1.


##### Withdrawals: User wants to withdraw select funds to their external wallets.
1. Alice: Hey, I want to move tokens out of my trading account.
1. Agent: Ok. Would you like to to move all the tokens from your trading account to your main account?
1. Alice: "No I only want to move 3 Near and 10 ETH" or "No I want to withdraw $200 worth."
1.


#### Withdrawals: User wants to withdraw some funds to their raincard
1.


##### Offboarding: User wants to withdraw all / close trading account
1. Alice: Hey, I want to move tokens out of my trading account.
1. Agent: Ok. Would you like to to move all the tokens from your trading account to your main account?
1. Alice: Yes
1. Agent: You have a total of 10 Near, 1 ETH, and .5 SOL in your trading account. If you plan to continue using Peerfolio, we need to keep a minimum of 5 Near on your trading account to keep it active. Do you plan to continue to use Peerfolio?
1. Alice: No
1. Agent: In a few seconds, you will be asked to review and approve a transaction to move X Near to alice.near.
User interacts with the wallet and then returns to Peerfolio.
1. Agent: ✅ I’ve moved your tokens back to your main account. You can view the transaction here if you’d like. Let me know if you want to create a new goal or check your portfolio details.


##### Less than Happy Path: User declines to act due to current market state
1. Alice says: actually my portfolio is underwater. I invested a total of $200 into a combination of Near and ETH. Both prices have since gone down, so I don't want to swap for stablecoins until my portfolio is worth more.
1. Agent says: Totally understood — I won’t suggest any swaps for now.


##### Less than Happy Path 2

