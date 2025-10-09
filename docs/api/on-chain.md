# API Reference: On-Chain Program

The `w3b2-solana-program` is an Anchor-based program that serves as the on-chain source of truth for all user and admin state.

This document provides a reference for its primary data structures (Accounts) and functions (Instructions).

## Core Accounts (PDAs)

The program uses two main types of Program Derived Addresses (PDAs) to store state on-chain.

### 1. `AdminProfile`

Represents a service provider. It stores the service's configuration and its earned balance.

-   **PDA Seeds**: `[b"admin", admin_authority.key().as_ref()]`
-   **Key Fields**:
    -   `authority: Pubkey`: The wallet that has permission to manage the profile.
    -   `communication_pubkey: Pubkey`: A public key used for off-chain communication or identification.
    -   `prices: Vec<PriceEntry>`: A list of services offered and their costs.
    -   `balance: u64`: The balance of funds earned by the service.

### 2. `UserProfile`

Represents a user's relationship with a single admin service. It holds the user's deposit for that specific service.

-   **PDA Seeds**: `[b"user", user_authority.key().as_ref(), admin_profile_pda.key().as_ref()]`
-   **Key Fields**:
    -   `authority: Pubkey`: The user's wallet that controls this profile.
    -   `admin_on_creation: Pubkey`: The `AdminProfile` PDA this profile is linked to.
    -   `communication_pubkey: Pubkey`: A public key for off-chain communication.
    -   `deposit_balance: u64`: The amount of lamports the user has deposited for this service.

## Instructions

All instructions are authenticated via a signature from the appropriate `authority` wallet.

### Admin Instructions

| Instruction              | Signer Authority | Description                                                               |
| ------------------------ | ---------------- | ------------------------------------------------------------------------- |
| `admin_register_profile` | Admin Wallet     | Creates the `AdminProfile` PDA for a new service.                         |
| `admin_update_comm_key`  | Admin Wallet     | Updates the `communication_pubkey` on the `AdminProfile`.                 |
| `admin_update_prices`    | Admin Wallet     | Overwrites the service price list, reallocating account space if needed.  |
| `admin_withdraw`         | Admin Wallet     | Withdraws earned funds from the `AdminProfile`'s internal `balance`.      |
| `admin_close_profile`    | Admin Wallet     | Closes the `AdminProfile` and refunds the rent lamports to the authority. |

### User Instructions

| Instruction            | Signer Authority | Description                                                                               |
| ---------------------- | ---------------- | ----------------------------------------------------------------------------------------- |
| `user_create_profile`  | User Wallet      | Creates a `UserProfile` PDA, linking the user to a specific admin service.                |
| `user_update_comm_key` | User Wallet      | Updates the `communication_pubkey` for a specific `UserProfile`.                          |
| `user_deposit`         | User Wallet      | Deposits lamports from the user's wallet into the `UserProfile` PDA's `deposit_balance`.  |
| `user_withdraw`        | User Wallet      | Withdraws unspent funds from the `UserProfile`'s `deposit_balance` back to the wallet.    |
| `user_close_profile`   | User Wallet      | Closes the `UserProfile` and refunds the entire balance (deposit + rent) to the user.     |

### Operational Instructions

These instructions facilitate the interaction loop between users and services.

| Instruction              | Signer Authority | Description                                                                                                                                                                                                                                                                                         |
| ------------------------ | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `user_dispatch_command`  | User Wallet      | The primary instruction for service usage. The user signs a transaction to call a specific `command_id`. If the command has a price (defined in the `AdminProfile`), the corresponding amount is transferred from the `UserProfile` deposit to the `AdminProfile` balance. Emits a `UserCommandDispatched` event. |
| `admin_dispatch_command` | Admin Wallet     | Allows an admin to send a command or notification to a user. This is a non-financial transaction used to emit an `AdminCommandDispatched` event, creating a verifiable on-chain record of the communication.                                                                                                |
| `log_action`             | User or Admin    | A generic instruction to log an off-chain action to the blockchain for auditing purposes. Emits an `ActionLogged` event.                                                                                                                         |

### The `payload` Field

Both `dispatch_command` instructions include an opaque `payload: Vec<u8>` field. This data is not interpreted by the on-chain program. It is passed through to the corresponding event, allowing off-chain applications to use the blockchain as a message bus for their own data structures.