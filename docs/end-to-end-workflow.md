# Tutorial: Full Workflow Example

This tutorial walks through a complete, realistic lifecycle of interactions within the W3B2 system. It covers registration, payments, moderation, and cleanup.

While the project includes an automated Python client that performs these actions, this guide explains the steps conceptually, referencing the gRPC calls a client application would make.

## The Scenario

We will follow the journey of an **Admin** (service provider) and a **User** (Alice).

1.  **Admin Registration**
    - **Action**: The Admin calls the `PrepareAdminRegisterProfile` gRPC method.
    - **Result**: The Admin signs and submits the transaction, creating their `AdminProfile` PDA on-chain.

2.  **User Registration**
    - **Action**: Alice calls `PrepareUserCreateProfile`, targeting the Admin's PDA.
    - **Result**: Alice signs and submits, creating her `UserProfile` PDA, which is permanently linked to the Admin's service.

3.  **User Deposit**
    - **Action**: Alice calls `PrepareUserDeposit` with an amount of `0.1 SOL`.
    - **Result**: Alice signs and submits, transferring `0.1 SOL` from her wallet to her `UserProfile` PDA. Her internal `deposit_balance` is now `0.1 SOL`.

4.  **Paid Command Execution**
    - **Action**: Alice wants to perform an action that costs `0.01 SOL`. Her client requests a signature from the Admin's oracle for this price. The oracle returns a signature. Alice then calls `PrepareUserDispatchCommand` with the command details and the oracle's signature.
    - **Result**: Alice signs and submits. The on-chain program verifies the oracle signature, then atomically transfers `0.01 SOL` from Alice's `UserProfile` PDA to the Admin's `AdminProfile` PDA.

5.  **Admin Bans User**
    - **Action**: The Admin decides to ban Alice and calls `PrepareAdminBanUser`, targeting Alice's `UserProfile` PDA.
    - **Result**: The Admin signs and submits. The `banned` flag in Alice's `UserProfile` is set to `true`.

6.  **User Requests Unban**
    - **Action**: The Admin has set an `unban_fee` of `0.005 SOL`. Alice calls `PrepareUserRequestUnban`.
    - **Result**: Alice signs and submits. The program verifies she is banned and has sufficient funds. It transfers `0.005 SOL` from her deposit to the Admin's balance and sets her `unban_requested` flag to `true`.

7.  **Admin Unbans User**
    - **Action**: The Admin's backend service sees the `UserUnbanRequested` event. After review, the Admin calls `PrepareAdminUnbanUser`.
    - **Result**: The Admin signs and submits. The `banned` and `unban_requested` flags in Alice's profile are set to `false`.

8.  **Cleanup**
    - **Action**: Alice decides to leave the service and calls `PrepareUserCloseProfile`.
    - **Result**: The `UserProfile` account is closed, and all remaining lamports (rent + deposit balance) are refunded to her wallet.

This entire flow is executed automatically by the Python example client when you run the project with Docker.