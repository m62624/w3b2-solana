# Tutorial: A Hybrid Web2/Web3 Service Workflow

This tutorial walks through a conceptual, end-to-end lifecycle of a user, Alice, interacting with a hypothetical high-throughput service (e.g., a non-custodial AI image generation service) built with the W3B2-Solana toolset.

The service has two main features:
1.  **Simple, On-Chain API**: A low-rate API to check account status, which costs a small, fixed amount of SOL.
2.  **High-Throughput Service**: A GPU-intensive image generation service that requires a direct, off-chain connection to a dedicated server for performance.

---

### 1. Setup: The Admin Registers

The service provider ("the Admin") creates their on-chain presence.

-   **Action**: The Admin calls `admin_register_profile`, providing a `communication_pubkey` that their backend will use for decrypting user requests.
-   **Result**: An `AdminProfile` PDA is created. The Admin then calls `admin_set_config` to set prices and fees.

---

### 2. A New User Arrives: Alice Creates Her Profile

Alice wants to use the service. She creates her on-chain profile, linking her wallet to the service.

-   **Action**: Alice calls `user_create_profile`, providing her own `communication_pubkey`.
-   **Result**: A `UserProfile` PDA is created, owned by Alice and linked to the Admin's service.

---

### 3. Funding the Account

Alice deposits `1 SOL` into her profile to pay for future actions.

-   **Action**: Alice calls `user_deposit` with `amount: 1000000000`.
-   **Result**: `1 SOL` is transferred from her wallet to her `UserProfile` PDA. Her `deposit_balance` is now `1 SOL`.

---

### 4. Simple On-Chain Interaction: Checking Status

Alice makes a simple, low-rate API call to check her account status. This is a standard on-chain transaction.

-   **Action**:
    1. The Admin's oracle signs a message containing the `command_id` (e.g., `1` for "check status") and `price` (e.g., `0.001 SOL`).
    2. Alice's client receives the signed data and calls `user_dispatch_command`.
-   **Result**: The program validates the oracle signature and transfers `0.001 SOL` from Alice's `UserProfile` to the `AdminProfile`. Her `deposit_balance` is now `0.999 SOL`.

---

### 5. Secure Handshake for a High-Throughput Service

Now, Alice wants to generate an AI image. This requires a powerful, dedicated GPU server. Transmitting the job data over Solana would be impossible. Instead, they use the secure handshake pattern.

-   **Action**:
    1.  **Client-Side**: Alice's client application generates a temporary, one-time secret for this session. It constructs a JSON payload containing the image prompt and this secret: `{"prompt": "a cat in a hat", "secret": "..."}`.
    2.  **Encryption**: The client encrypts this JSON payload using the Admin's on-chain `communication_pubkey`.
    3.  **On-Chain Handshake**: Alice calls `user_dispatch_command`. The `price` is `0` (as the real cost is tied to off-chain compute), and the encrypted JSON is placed in the `payload` field.
-   **Off-Chain Reaction**:
    1.  **Event Listening**: The Admin's backend, listening via the `w3b2-solana-connector`, receives the `UserCommandDispatched` event containing the encrypted payload.
    2.  **Decryption**: The backend uses its private communication key to decrypt the payload, revealing the image prompt and Alice's one-time secret.
    3.  **Connection**: The backend now knows what Alice wants to do. It provisions a GPU server and establishes a direct, private off-chain connection with Alice's client (e.g., via WebSocket), authenticating her with the one-time secret.
-   **Result**: Alice's client and the Admin's GPU server now have a direct, high-speed connection for the image generation job, completely bypassing the blockchain. The on-chain transaction serves as a verifiable audit log that this session was initiated.

---

### 6. Logging Off-Chain Progress

The AI generation is an off-chain process. To maintain an audit trail, the Admin's service logs key milestones to the blockchain.

-   **Action**:
    1.  The GPU server finishes rendering the image and sends it to Alice over the direct off-chain channel.
    2.  The server then calls `log_action` with `session_id` (linking it to the initial handshake) and `action_code` (e.g., `200` for "OK").
-   **Result**: An `OffChainActionLogged` event is emitted on-chain. This creates an immutable record that the service verifiably completed the off-chain work requested in the handshake. This is crucial for dispute resolution and transparency.

---

### 7. Lifecycle: Ban, Appeal, and Withdrawal

The tutorial proceeds as before with the ban/unban cycle:
-   The Admin bans Alice with `admin_ban_user`.
-   Alice's next `user_dispatch_command` fails with a `UserIsBanned` error.
-   Alice appeals by paying a fee with `user_request_unban`.
-   The Admin reviews the appeal (notified by the `UserUnbanRequested` event) and unbans her with `admin_unban_user`.
-   Finally, Alice withdraws her remaining funds with `user_withdraw` and closes her account with `user_close_profile` to reclaim the rent.

This complete workflow demonstrates how the toolset combines the security of on-chain transactions with the performance of off-chain services.