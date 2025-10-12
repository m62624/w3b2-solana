# Tutorial: Full End-to-End Workflow

This tutorial walks through a complete, conceptual lifecycle of a user interacting with a service powered by the W3B2 toolset. It follows the user from their initial registration to performing actions, getting banned, and finally closing their account.

Each step will mention the on-chain instruction that a client application (using a library like `anchorpy` or `@coral-xyz/anchor`) would call to initiate the action.

---

### 1. Setup: The Admin Registers

Before any users can join, the service provider (the "Admin") must create their on-chain presence by creating an `AdminProfile`.

-   **Action**: The Admin calls the gateway to create their `AdminProfile`.
-   **On-Chain Instruction**: `admin_register_profile`
-   **Result**: An `AdminProfile` PDA is created on-chain. The admin can now configure it, for example, by calling `admin_set_config` to set an `unban_fee`.

---

### 2. A New User Arrives: Profile Creation

A new user, Alice, decides to use the service. Her first step is to create her own on-chain `UserProfile`, which links her wallet to the admin's service.

-   **Action**: Alice's client application calls the gateway to create her `UserProfile`.
-   **On-Chain Instruction**: `user_create_profile`
-   **Result**: An on-chain `UserProfile` PDA is created, owned by Alice but linked to the admin's profile.

---

### 3. Funding the Account: Making a Deposit

Alice needs to pay for actions. To do this, she pre-funds her account by depositing `1 SOL`.

-   **Action**: Alice authorizes a deposit transaction.
-   **On-Chain Instruction**: `user_deposit` (with `amount: 1000000000`)
-   **Result**: `1 SOL` is transferred from Alice's wallet to her `UserProfile` PDA. Her on-chain `deposit_balance` is now `1 SOL`. The funds are still hers, but are now available for the program to use for payments she authorizes.

---

### 4. Using the Service: A Paid Command

Alice now performs a paid action, which costs `0.1 SOL`. The service's backend oracle signs the price and command details.

-   **Action**: Alice's client builds a transaction containing the oracle's signature verification and the command itself, then signs and sends it.
-   **On-Chain Instruction**: `user_dispatch_command` (with `price: 100000000`)
-   **Result**: The on-chain program verifies the oracle's signature. It then atomically transfers `0.1 SOL` from Alice's `UserProfile` PDA to the `AdminProfile` PDA. Alice's `deposit_balance` is now `0.9 SOL`.

---

### 5. A Misstep: The Ban

The admin determines that Alice has violated the terms of service and bans her.

-   **Action**: The Admin sends a transaction to ban Alice's profile.
-   **On-Chain Instruction**: `admin_ban_user` (targeting Alice's `UserProfile` PDA)
-   **Result**: The `banned` flag on Alice's `UserProfile` is set to `true`.

---

### 6. The Consequence: A Failed Action

Alice, perhaps unaware she's been banned, tries to perform another paid action.

-   **Action**: She attempts to call `UserDispatchCommand` again.
-   **On-Chain Instruction**: `user_dispatch_command`
-   **Result**: The transaction **fails**. The on-chain program checks the `banned` flag at the beginning of the instruction and returns a `UserIsBanned` error. No funds are moved.

---

### 7. The Appeal: Requesting an Unban

Alice realizes she is banned and decides to appeal. The admin has set a `0.05 SOL` fee for unban requests.

-   **Action**: Alice calls the instruction to request an unban.
-   **On-Chain Instruction**: `user_request_unban`
-   **Result**: The program checks that Alice has sufficient funds. It transfers `0.05 SOL` from her `deposit_balance` to the admin's balance and sets the `unban_requested` flag on her profile to `true`. Alice's balance is now `0.85 SOL`.
-   **Important**: Alice is **still banned**. This action only signals her request for a manual review.

---

### 8. The Verdict: The Unban

The admin's backend is notified of the `UserUnbanRequested` event via the gRPC event stream. The admin reviews the case and decides to grant the appeal.

-   **Action**: The admin calls the instruction to unban Alice.
-   **On-Chain Instruction**: `admin_unban_user`
-   **Result**: The `banned` and `unban_requested` flags on Alice's `UserProfile` are set back to `false`. Alice can now use the service again.

---

### 9. Moving On: Withdrawing Funds and Closing

Alice decides to stop using the service. She withdraws her remaining balance and closes her account.

1.  **Withdrawal**:
    -   **Action**: Alice withdraws her remaining `0.85 SOL`.
    -   **On-Chain Instruction**: `user_withdraw` (with `amount: 850000000`)
    -   **Result**: Her entire remaining `deposit_balance` is transferred from the `UserProfile` PDA back to her wallet.

2.  **Closure**:
    -   **Action**: Alice closes her now-empty profile to reclaim the rent she paid for its on-chain storage.
    -   **On-Chain Instruction**: `user_close_profile`
    -   **Result**: The `UserProfile` PDA is deleted from the blockchain, and the lamports held for rent are refunded to Alice's wallet. Her lifecycle with the service is now complete.