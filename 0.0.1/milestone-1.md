#### Milestone 1

<b>Audience:</b> Crypto Native Users
<b>Overview:</b> After a brief chat with the Peerfolio Agent, Alice instructs the agent to setup a limited access account that the agent uses to trade on her behalf to realize her allowance goal of $25.00 USDT.


##### User Journey Happy Path - Onbaording & Goal Creation
1. Alice opens the Peerfolio UI at peerfolio.app
1. Agent delivers it's welcome message:  I'm Peerfolio, your financial assistant for managing and growing your crypto portfolio. I can help you with account details, real-time market prices of tokens, setting and tracking your financial goals, and recommending token swaps to achieve those goals. To get started, please tell me your near account Id.
1. Alice shares that her info is alice.near
1. Agent explains its ability to trade autonomously to realize gains as stablecoins: Your account balance on alice.near is 100 Near, with 1 Near currently priced at $3.18, your portfolio is worth $318.00. If your porfolio grows by 5%, you could swap $15 into stablecoins without reducing your portfolio below today's market value. Would you like me to set this as a swap goal?
1. Alice says: No, what else can you do for me?
1. Agent responds with the options: check your account details, check market prices, help you set an swap goal, recommend tokens to swap to realize your goals.
1. Alice says: Ok, help me realize that weekly swap goal.
1. Agent says: Ok great. I've saved a swap goal to realize $15 in USDC or USDT only if your portfolio grows by at least 5%, to at least reach $318.  Please confirm that this meets your expectations.
1. Alice: yes.
1. Agent says: In order to securely trade on your behalf I need your permission to transfer some tokens into a limited access trading account that you will own. It is from this trading account that I will be able to access your tokens and initiate transfers on your behalf. Shall we go ahead and create this limited access trading account, or would you like to know more details of how this works?
1. Alice says: I'd like to know more.
1. Agent says: Please dig into the technical deails @ <github link to beneivo labs auth_proxy readme> and post any questions to the Peerfolio team at <telegram dev support group TBD>
1. Alice says: Ok I'm comfortable with that. Go head and setup my trading account.
1. Agent: displays a Connect Wallet Button, which asks Alice to sign a message.
1. Agent UI: on the callback from the signmessage action, the Agent says: Now let's determine the types and quantities of tokens to move into your trading account.  Peerfolio will only have access to the tokens on your trading account. All other tokens on alice.near will be untouchable. Would you like to fund your trading account with 10%, 15% or 20% of the 100 Near on your account? (Agent repeats the same quesiton for each supported token type: ETH, SOL, etc.)
1. Alice: I'm ok with 10
1. Agent UI: Ok understood. (Agent adds this trading % into state) In a moment you will be asked to approve two transactions. The 1st asks to create the trading account as a sub account of your main account called agent.alice.near with X Near transfered to your trading account. This trading account contains the logic that allows you to control where and how Peerfolio is allowed to trade on your behalf.  The second transaction asks to allow Peerfolio to request signatures to approve transactions on your trading account.
1. Agent UI begins a sequence of asking the user to approve 2 transactions.
1. Agent: First, in a few seconds, you will be prompted to review and approve a transaction requesting to create your trading account at agent.alice.near with a deposit of X Near (where X =  ~.02 Near required to create the account + Y Near to secure storing the proxy contract + their chosen trading %)
1. Throughout this process the agent should maintain a state that allows us to recovery and restart from the appropriate step should any transaction fail or not be approved. To be stored in state: the hash of the contract deployed at the trading account. This way, if alice.near already has the expected limited access account but there was a failure in the deployment of the proxy contract, we could proceed directly to the proxy account deployment . Also to be stored in the state: the derived MPC public key for agent.alice.near; such that we can be aware of any failures in the 2nd transaction.
1. After the 1st transaction is approved, in the background, Peerfolio
makes a view call ala contracts/scripts/derive_mpc_key.sh to get the MPC public key which is needed in the 3rd txn.
1. Agent: Now that your trading account has been created, in a few seconds you will be asked to review and approve a transaction to grant Peerfolio the ability to approve transactions which are limited to  agent.alice.near, and calls required to support Near, USDT, and USDC swaps on near-intents.
1. After all transactions have completed.  The UI throws some confetti or otherwise lets the user know that they are amazing and they are ready to enjoy Peerfolio.
1. Agent: Great, let's review your goals: we've set a swap goal to realize $15 in USDC or USDT only if the value of your Near grows by at least 5%, to at least reach $318. Would you like to make any change?
1. Alice: No
1. Agent: If you would like to recieve notifications on the status of your swap goals, please share your 10-digit USA-based phone number.



##### Offboarding: Existing User wants to stop using Peerfolio.
1. Alice: Hey I'm done using this app.
1. Agent: Ok. Would you like to to move all the tokens from your trading account to your main account?
1. Alice: yes
1. Agent: You have a total of 10 Near on your trading account. If you plan to continue using Peerfolio, we need to keep a minimum of 5 Near on your trading account to keep it active. Do you plan to continue to use Peerfolio?
1. Alice: No
1. Agent: Just to confirm, Are you sure that you want to close your trading account with Peerfolio?
1. Alice: Yes dammit.
1. Agent: In a few seconds, you will be asked to review and approve a transaction to close your trading account and move all Near to alice.near.
1. User approves the wallet transaction and then returns to Peerfolio.
1. Agent: Hello! I'm Peerfolio, your financial assistant for managing and growing your crypto portfolio. I can help you with account details, real-time market prices of tokens, setting and tracking your financial goals, and recommending token swaps to achieve those goals. How can I assist you today?


##### Offboarding: Existing User wants to defund trading account
1. Alice: Hey, I want to move tokens out of my trading account.
1. Agent: Ok. Would you like to to move all the tokens from your trading account to your main account?
1. Alice: Yes or "No I only want to move 3 Near"
1. Agent: You have a total of 10 Near on your trading account. If you plan to continue using Peerfolio, we need to keep a minimum of 5 Near on your trading account to keep it active. Do you plan to continue to use Peerfolio?
1. Alice: Yes of course
1. Agent: In a few seconds, you will be asked to review and approve a transaction to move X Near to alice.near.
1. User interacts with the wallet and then returns to Peerfolio.


##### Less than Happy Path: User declines to act due to current market state
5. Alice says: actually my portfolio is underwater. I invested a total of $200 into a combination of Near and ETH. Both prices have since gone down, so I don't want to swap for stablecoins until my portfolio is worth more.
6. Agent says: ....



##### Less than Happy Path: User wants unsupported tokens
5. Alice says: actually my portfolio is underwater. I invested a total of $200 into a combination of Near and ETH. Both prices have since gone down, so I don't want to swap for stablecoins until my portfolio is worth more.
6. Agent says: Understood. If you are interested in aquiring any other tokens, please let me know what types of tokens and what quantities.
7. Alice: I would buy more XRP or SHITZU if the prices are within a certain range.
8. Agent: We currently only support NEAR, USDT, and USDC, but I've sent a note of your request to our product team.  If you'd like to be the frist to know when these are supported, please join our telegram channel @ <benevio labs telegram announcements channel>




##### Less than Happy Path 2
5. Alice says: Before we do anything on mainnet. I'd like to get a better understanding on how you securely trade on my behalf by first running some test trades. Can we do that?
6. Agent says: Yes! Please visit our testnet demo @ sandbox.peerfolio.app where you can get a feel for how we approach securely trading on your behalf via a limited access trading account from the safe confines of a sandbox enfvrionment where there is no real value associated to testnet tokens. Please use this testnet sandbox to setup and monitor hypothetical trading situations, and observe that we're doing a useful job before coming back here for the mainnet Peerfolio experience.

##### Less than Happy Path 3
8. Agent says: Unfortunately, we can only create a goal for a future swap if your portfolio has at least $10 worth of assets. We can revist creating a swap goal as soon as you have more assets available to trade.  Would you like to explore aquiring any other tokens?


