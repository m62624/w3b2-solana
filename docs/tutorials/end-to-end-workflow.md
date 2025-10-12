# Tutorial: A User's End-to-End Workflow

This tutorial walks through a complete, conceptual lifecycle of a user interacting with a service powered by the W3B2 toolset. It follows the user from their initial registration to performing actions, getting banned, requesting an unban, and finally closing their account.

Each step will mention the conceptual gRPC method that a client application would call to initiate the action.

---

### 1. Setup: The Admin Registers

Before any users can join, the service provider (the "Admin") must create their on-chain presence.

-   **Action**: The Admin calls the gateway to create their `AdminProfile`.
-   **gRPC Method**: `PrepareAdminRegisterProfile` -> `SubmitTransaction`
-   **Result**: An `AdminProfile` PDA is created on-chain. The admin can now configure it, for example, by setting an `unban_fee`.

---

### 2. A New User Arrives: Profile Creation

A new user, Alice, decides to use the service. Her first step is to create her own on-chain profile, which links her wallet to the admin's service.

-   **Action**: Alice's client application calls the gateway to create her `UserProfile`.
-   **gRPC Method**: `PrepareUserCreateProfile` -> `SubmitTransaction`
-   **Result**: An on-chain `UserProfile` PDA is created, owned by Alice but linked to the admin's profile.

---

### 3. Funding the Account: Making a Deposit

Alice needs to pay for actions. To do this, she pre-funds her account by depositing 1 SOL.

-   **Action**: Alice authorizes a deposit transaction.
-   **gRPC Method**: `PrepareUserDeposit` (with `amount: 1000000000`) -> `SubmitTransaction`
-   **Result**: 1 SOL is transferred from Alice's wallet to her `UserProfile` PDA. Her on-chain `deposit_balance` is now 1 SOL. The funds are still hers, but are now available for the program to use for payments she authorizes.

---

### 4. Using the Service: A Paid Command

Alice now performs a paid action, which costs 0.1 SOL. The service's backend oracle signs the price and command details.

-   **Action**: Alice signs the transaction prepared by the gateway, which includes the oracle's signature verification and the command itself.
-   **gRPC Method**: `PrepareUserDispatchCommand` (with `price: 100000000`) -> `SubmitTransaction`
-   **Result**: The on-chain program verifies the oracle's signature. It then atomically transfers 0.1 SOL from Alice's `UserProfile` PDA to the `AdminProfile` PDA. Alice's `deposit_balance` is now 0.9 SOL.

---

### 5. A Misstep: The Ban

The admin determines that Alice has violated the terms of service and decides to ban her.

-   **Action**: The Admin calls the gateway to ban Alice's profile.
-   **gRPC Method**: `PrepareAdminBanUser` (targeting Alice's `UserProfile` PDA) -> `SubmitTransaction`
-   **Result**: The `banned` flag on Alice's `UserProfile` is set to `true`.

---

### 6. The Consequence: A Failed Action

Alice, unaware she's been banned, tries to perform another paid action.

-   **Action**: She attempts to call `UserDispatchCommand` again.
-   **gRPC Method**: `PrepareUserDispatchCommand` -> `SubmitTransaction`
-   **Result**: The transaction **fails**. The on-chain program checks the `banned` flag at the beginning of the instruction and returns a `UserIsBanned` error. No funds are moved.

---

### 7. The Appeal: Requesting an Unban

Alice realizes she is banned and decides to appeal. The admin has set a 0.05 SOL fee for unban requests.

-   **Action**: Alice calls the instruction to request an unban.
-   **gRPC Method**: `PrepareUserRequestUnban` -> `SubmitTransaction`
-   **Result**: The program checks that Alice has sufficient funds. It transfers 0.05 SOL from her `deposit_balance` to the admin's balance and sets the `unban_requested` flag on her profile to `true`. Alice's balance is now 0.85 SOL.
-   **Important**: Alice is **still banned**. This action only signals her request for a manual review.

---

### 8. The Verdict: The Unban

The admin's backend is notified of the `UserUnbanRequested` event. The admin reviews the case and decides to grant the appeal.

-   **Action**: The admin calls the instruction to unban Alice.
-   **gRPC Method**: `PrepareAdminUnbanUser` -> `SubmitTransaction`
-   **Result**: The `banned` and `unban_requested` flags on Alice's `UserProfile` are set back to `false`. Alice can now use the service again.

---

### 9. Moving On: Withdrawing Funds and Closing

Alice decides to stop using the service. She wants to withdraw her remaining balance and close her account.

1.  **Withdrawal**:
    -   **Action**: Alice withdraws her remaining 0.85 SOL.
    -   **gRPC Method**: `PrepareUserWithdraw` (with `amount: 850000000`) -> `SubmitTransaction`
    -   **Result**: Her entire remaining `deposit_balance` is transferred from the `UserProfile` PDA back to her wallet.

2.  **Closure**:
    -   **Action**: Alice closes her now-empty profile to reclaim the rent she paid for its on-chain storage.
    -   **gRPC Method**: `PrepareUserCloseProfile` -> `SubmitTransaction`
    -   **Result**: The `UserProfile` PDA is deleted from the blockchain, and the lamports held for rent are refunded to Alice's wallet. Her lifecycle with the service is now complete.