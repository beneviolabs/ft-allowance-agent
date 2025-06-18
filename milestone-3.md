#### WIP Milestone 3

<b>Audience:</b> Mainstream Consumers.
<b>Overview:</b>

##### Major enhancements // TODO Turn these info fully scoped issues
  - launch multiplayer functionality
  - launch support for mainstream consumers:
    - via a passkey-based onboarding flow. create a flow diagram to vet the use of webAuthn and fully plan this architecture.
  - Support ETH, SOL, XRP, and BTC in the deposit to token aquisition flow
  - Expand the token acquisition flow to
asset allocation buckets, e.g. Highest Market Cap (BTC), AI Focused (NEAR, ICP, TAO), DeFi & Prediction Markets (ETH), and NFT/Memecoins
  - Offramp: Alpha release of Raincard integration. Design the required onboarding and technical designs for Raincard integration
  - build a dashboard that shows:
    - the conversion rate from visit to successful onramp deposit
    - the execution rate of saved growth goals showing the time the goal was created, and date, amount of swap execution



##### Support ETH, SOL, XRP, and BTC in the deposit to token aquisition flow
1. categorize into themes asset objects around these assets + Near, to which we can add more assets in the future. Key=token_ticker Value=percentage_allocation. USDC should be included here.
1. Is there any legal or UX benefit to leting users modify the percentages?
1.

#### Offramp: User wants to withdraw some funds to their raincard
1.

#### Offramp: User wants to withdraw some funds to their CEX address(es)
1.

####  Sandbox flow:
1. Alice says: Before we do anything on mainnet. I'd like to get a better understanding on how you securely trade on my behalf by first running some test trades. Can we do that?
1. Agent says: Yes! Please visit our testnet demo @ sandbox.peerfolio.app where you can get a feel for how we approach securely trading on your behalf via a limited access trading account from the safe confines of a sandbox enfvrionment where there is no real value associated to testnet tokens. Please use this testnet sandbox to setup and monitor hypothetical trading situations, and observe that we're doing a useful job before coming back here for the mainnet Peerfolio experience.


####  Less than Happy Path: User wants unsupported tokens
1. Alice says: actually my portfolio is underwater. I invested a total of $200 into a combination of Near and ETH. Both prices have since gone down, so I don't want to swap for stablecoins until my portfolio is worth more.
1. Agent says: Understood. If you are interested in aquiring any other tokens, please let me know what types of tokens and what quantities.
1.  Do we need to support this flow? This may become lower needed priority for our mainstream consumers.

