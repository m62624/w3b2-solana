# API Reference: On-Chain Program

The `w3b2-solana-program` is the core smart contract that enforces all rules and manages all state for the toolset. This document provides a reference for its data structures and instructions.

For a conceptual overview of how this fits into the larger system, see the **[Architecture](../architecture.md)** guide.

## On-Chain State Accounts

The program uses two primary Program Derived Address (PDA) accounts to store state.

### `AdminProfile`

This PDA represents a service provider ("Admin"). Its address is deterministically derived from the admin's wallet key, ensuring one profile per admin.

-   **PDA Seeds**: `[b"admin", authority.key().as_ref()]`
-   **Key Fields**:
    -   `authority: Pubkey`: The admin's wallet, with sole authority to manage the profile.
    -   `oracle_authority: Pubkey`: The key trusted to sign price information for paid commands.
    -   `communication_pubkey: Pubkey`: A public key for off-chain communication.
    -   `balance: u64`: The internal treasury where collected fees accumulate.
    -   `unban_fee: u64`: A configurable fee a user must pay to request an unban review.
    -   `timestamp_validity_seconds: i64`: The window (in seconds) for which an oracle signature is valid.

### `UserProfile`

This PDA represents a user's relationship with a specific admin service. Its address is derived from both the user's wallet and the admin's profile key, creating a unique link for each user-service pair.

-   **PDA Seeds**: `[b"user", authority.key().as_ref(), admin_profile.key().as_ref()]`
-   **Key Fields**:
    -   `authority: Pubkey`: The user's wallet, with sole authority to manage the profile.
    -   `admin_profile_on_creation: Pubkey`: An immutable link to the `AdminProfile` PDA this profile is associated with.
    -   `deposit_balance: u64`: The user's prepaid balance for this service.
    -   `banned: bool`: A flag indicating if the user is banned by the admin.
    -   `unban_requested: bool`: A flag indicating the user has paid the fee and requested an unban review.

## Instruction Set

The program exposes a set of instructions for managing profiles and dispatching commands. All instructions are authenticated via a signature from the appropriate `authority` wallet.

### Admin Instructions

| Instruction                | Description                                                                                             |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `admin_register_profile`   | Initializes a new `AdminProfile` PDA for a service provider.                                            |
| `admin_set_config`         | Updates configuration, such as the `oracle_authority`, `unban_fee`, or `timestamp_validity_seconds`.    |
| `admin_withdraw`           | Withdraws earned funds from the `AdminProfile`'s internal `balance`.                                    |
| `admin_ban_user`           | Bans a user, preventing them from using the service.                                                    |
| `admin_unban_user`         | Unbans a user, restoring their access. See the "Request for Review" model in the Architecture guide.    |
| `admin_dispatch_command`   | Sends a non-financial command or notification to a user, emitting an on-chain event.                    |
| `admin_close_profile`      | Closes the `AdminProfile` account and refunds the rent lamports to the admin.                           |

### User Instructions

| Instruction                | Description                                                                                             |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `user_create_profile`      | Creates a `UserProfile` PDA, linking a user's wallet to a specific admin service.                       |
| `user_deposit`             | Deposits lamports into the `UserProfile`'s `deposit_balance` to pre-fund future payments.               |
| `user_withdraw`            | Withdraws unspent funds from the `deposit_balance`.                                                     |
| `user_update_comm_key`     | Updates the `communication_pubkey` for the user's profile.                                              |
| `user_request_unban`       | Allows a banned user to pay the `unban_fee` and request an unban review from the admin.                 |
| `user_close_profile`       | Closes the `UserProfile` and refunds all remaining lamports (rent and deposit balance) to the user.     |

### Operational Instructions

| Instruction                | Description                                                                                             |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `user_dispatch_command`    | **Primary operational instruction.** A user calls a service's command, verifying a signed price from the admin's oracle and transferring payment if applicable. |
| `log_action`               | A generic instruction to log a significant off-chain action to the blockchain for an audit trail.       |

For the most detailed information on each instruction's arguments, required accounts, and emitted events, please refer to the Rustdoc comments in the `w3b2-solana-program` source code.