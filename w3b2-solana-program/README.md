# W3B2 Solana Program

This crate contains the core on-chain smart contract for the W3B2-Solana toolset. Built with Anchor, it provides a secure and verifiable framework for off-chain services to interact with the Solana blockchain. The program enables services to manage user profiles, handle payments, and dispatch commands in a non-custodial manner, where users always retain control of their funds and sign all transactions with their own wallets.

## Core Concepts

- **Admin & User Profiles**: The program establishes two primary PDA account types:
    - `AdminProfile`: Represents the service provider. It holds configuration like the oracle key and serves as a treasury for collected fees.
    - `UserProfile`: Represents an end-user's relationship with a specific service. It holds the user's pre-paid deposit balance.

- **Non-Custodial Payments**: Users deposit funds into their own `UserProfile` PDA. Payments for services are transferred from the user's profile to the admin's profile only upon the user's explicit, signed approval via the `user_dispatch_command` instruction.

- **Off-Chain Oracle**: For dynamic pricing and authorization, the program uses an off-chain oracle pattern. The service's backend signs payment details (price, command, timestamp), and the on-chain program verifies this signature before processing a payment. This keeps business logic flexible and off-chain, while keeping value transfer secure and on-chain.

- **Event-Driven Architecture**: The program emits detailed events for every significant action. Off-chain clients, like the `w3b2-solana-connector`, can listen for these events to synchronize state and trigger backend processes.

## Instruction Set

The program exposes a set of instructions for managing profiles and dispatching commands.

### Admin Instructions

| Instruction                | Description                                                                    |
| -------------------------- | ------------------------------------------------------------------------------ |
| `admin_register_profile`   | Initializes a new `AdminProfile` PDA for a service provider.                   |
| `admin_set_config`         | Updates configuration, such as the `oracle_authority` or `unban_fee`.          |
| `admin_withdraw`           | Withdraws earned funds from the `AdminProfile`'s internal balance.             |
| `admin_ban_user`           | Bans a user, preventing them from using the service.                           |
| `admin_unban_user`         | Unbans a user, restoring their access.                                         |
| `admin_dispatch_command`   | Sends a non-financial command or notification to a user, emitting an event.    |
| `admin_close_profile`      | Closes the `AdminProfile` account and refunds the rent to the admin.           |

### User Instructions

| Instruction                | Description                                                                    |
| -------------------------- | ------------------------------------------------------------------------------ |
| `user_create_profile`      | Creates a `UserProfile` PDA, linking a user's wallet to a specific admin.      |
| `user_deposit`             | Deposits lamports into the `UserProfile`'s `deposit_balance`.                  |
| `user_withdraw`            | Withdraws unspent funds from the `deposit_balance`.                            |
| `user_update_comm_key`     | Updates the `communication_pubkey` for the user's profile.                     |
| `user_request_unban`       | Allows a banned user to pay the `unban_fee` and request an unban.              |
| `user_close_profile`       | Closes the `UserProfile` and refunds all lamports (rent & deposit) to the user.|

### Operational Instructions

| Instruction                | Description                                                                    |
| -------------------------- | ------------------------------------------------------------------------------ |
| `user_dispatch_command`    | **Primary operational instruction.** A user calls a service's command, verifying a signed price from the admin's oracle and transferring payment. |
| `log_action`               | A generic instruction to log an off-chain action for an on-chain audit trail.  |

For more detailed information on each instruction's arguments, account requirements, and emitted events, please refer to the Rustdoc comments within the source code (`src/lib.rs`).