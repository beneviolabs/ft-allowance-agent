# Test prompts

Format:

```
## n.n Test name
>
    -> prompt
    <- expected response
```

## 1. Near account

### 1.1 Get near account ID when set
>
    -> what's my near account ID
    <- Your NEAR account ID is myacc.near.

### 1.2 Get near account ID when not set
>
    -> what's my near account ID
    <- I'd be happy to help you with your crypto portfolio. However, I don't have access to your NEAR account ID at the moment. Could you please provide it to me so I can better assist you?

### 1.3 Set valid near account ID
>
    -> set my near account ID to myacc.near
    <- Saved your NEAR account ID: myacc.near

>
    -> myacc.near
    <- Saved your NEAR account ID: myacc.near

### 1.4 Set invalid near account ID
>
    -> set my near account ID to blorbo.
    <- Please provide a valid NEAR account ID.

## 2. Portfolio Goals

### 2.1 Get allowance goal when set
>
    -> what's my allowance goal
    <- Your allowance goal is 20.

### 2.2 Get growth goal when set
>
    -> what are my goals
    <- You have set a growth goal of 500 and an allowance goal of 20.

### 2.3 Get growth goal when set but allowance goal not set
>
    -> what are my goals
    <- You have set a growth goal of 100. You haven't set an allowance goal yet. Would you like to set one?

### 2.4 Get goals when none set
>
    -> what are my goals
    <- Hello! I'm Divvy, your financial assistant for managing and growing your crypto portfolio. I see that you haven't set any goals for your portfolio yet. Would you like to set a growth goal or an allowance goal? I'm here to help you achieve your financial objectives.

### 2.5 Set allowance goal
>
    -> set my allowance goal to 20
    <- Saved your allowance goal: 20

### 2.6 Set growth goal
>
    -> set my growth goal to 500
    <- Saved your growth goal: 500

### 2.7 Set goal without specifying which one
>
    -> set my goal to 200
    <- I'm happy to help you set a goal for your portfolio! However, I need to clarify what type of goal you would like to set. Would you like to set a growth goal or an allowance goal?

    #### Or

    -> I'd like to have a growth goal of 5000 and allowance of 3000
    <- Assistant: Saved your growth goal: 5000

    -> what are my goals?
    <- Assistant: It seems I made a mistake earlier. You had set an allowance goal of 300. Here are your current goals:

        * Growth goal: 5000
        * Allowance goal: 300

    -> what is my allowance goal?
    <- Assistant: Your allowance goal is actually 3000, which you set earlier.

## 3. Account Balance

### 3.1 Get account balance when near account ID not set
>
    -> what's my balance?
    <- I'm happy to help you with your account balance. Unfortunately, I don't have your NEAR account ID on file. Can you please provide it to me so I can look up your balance?

### 3.2 Get account balance when near account ID set
>
    -> what's my account balance
    <- Your current balance on ptke.near is 3.14 NEAR.

### 3.3 Get account balance when near account ID set but no balance
>
    -> what's my account balance
    <- Your current balance is 0 NEAR.

## 4. Realize my allowance goal

Your allowance and growth goals must first be set.  Note: there are currently a harded coded quantity of BTC, SOL, NEAR, and ETH tokens that will be used to suggest quantities of tokens to swap.

 -> recommend token swaps to realize my allowance goal
<- Assistant: Considering your options with a preference for holding BTC...
Assistant: I can suggest a token swap to help you achieve your allowance goal. Based on your current portfolio and market conditions, I recommend swapping a portion of your NEAR for a stablecoin, such as USDT. This swap would allow you to realize a gain of approximately 29.96 NEAR, which is equivalent to your targe

TODO - fix the LLMs interpretation of the recommended swap results. Here's an example of when the LLM mispresented the quantities and types of tokens which were calculated:

our function returned: {'BTC': 0.025131535330388054, 'ETH': 0.15804649889455302, 'SOL': 0.23546598135175315, 'NEAR': 10.730203309033069}

and the LLM responded with:
Assistant: Considering your options with a preference for holding BTC...
Assistant: Based on the current market prices, I recommend swapping 0.23546598 NEAR for SOL to realize your allowance goal of 3000.




