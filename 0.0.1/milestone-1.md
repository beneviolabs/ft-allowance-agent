#### Milestone 1

<b>Audience:</b> Crypto Native Users
<b>Overview:</b> After a brief chat with the Peerfolio Agent, Alice instructs the agent to setup a limited access account that the agent uses to trade on her behalf to realize her allowance goal of $25.00 USDT.


##### User Journey Happy Path
1. Alice opens the Peerfolio UI at peerfolio.app
1. Agent delivers it's welcome message and asks for her near account Id.
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
1. Agent UI: Ok understood. In a moment you will be asked to approve three transactions. The 1st asks to create the trading account as a sub account of your main account called agent.alice.near with 10 Near transfered to your trading account. The 2nd transaction asks to deploy the limited access contract to your trading account. This contract contains the logic that allows you to control where and how Peerfolio is allowed to trade on your behalf. The final transaction asks to allow Peerfolio to request signatures to approve transactions on your trading account.
1. Agent UI begins a sequence of asking the user to approve 3 transactions.
1. After the 2nd transaction is approved, in the background, Peerfolio
makes a view call ala contracts/scripts/derive_mpc_key.sh to get the MPC public key which is needed in the 3rd txn.
1. After all transactions have completed.  The UI throws some confetti or otherwise lets the user know that they are amazing and they are ready to enjoy Peerfolio.


##### Less than Happy Path 1
5. Alice says: actually my portfolio is underwater. I invested a total of $200 into a combination of Near and ETH. Both prices have since gone down, so I don't want to swap for stablecoins until my portfolio is worth more.
6. Agent says: Understood. If you are interested in aquiring any other tokens, please let me know what types of tokens and what quantities.
7. Alice: I would buy more XRP or SHITZU if the prices are within a certain range.
8. Agent: We currently only support NEAR, USDT, and USDC, but I've sent a note of your request to our product team.  If you'd like to be the frist to know when these are supported, please join our telegram channel @ <benevio labs telegram annoucements channel>


##### Less than Happy Path 2
5. Alice says: Before we do anything on mainnet. I'd like to get a better understanding on how you securely trade on my behalf by first running some test trades. Can we do that?
6. Agent says: Yes! Please visit our testnet demo @ sandbox.peerfolio.app where you can get a feel for how we approach securely trading on your behalf via a limited access trading account from the safe confines of a sandbox enfvrionment where there is no real value associated to testnet tokens. Please use this testnet sandbox to setup and monitor hypothetical trading situations, and observe that we're doing a useful job before coming back here for the mainnet Peerfolio experience.
