# W3B2 Solana Program

This crate contains the core on-chain smart contract for the W3B2 toolset. Built with Anchor, it provides a secure and verifiable framework for Web2 services to interact with the Solana blockchain. The program enables services to manage user profiles, handle payments, and dispatch commands in a non-custodial manner, where users always retain control of their funds and sign all transactions with their own wallets.

## Core Concepts

- **Admin & User Profiles**: The program establishes two primary PDA account types:
    - `AdminProfile`: Represents the service provider (the "Admin"). It holds configuration like the oracle key and serves as a treasury for collected fees.
    - `UserProfile`: Represents an end-user's relationship with a specific service. It holds the user's pre-paid deposit balance for that service.

- **Non-Custodial Payments**: Users deposit funds into their own `UserProfile` PDA, which is controlled by the program. Payments for services are transferred from the user's profile to the admin's profile only upon the user's explicit, signed approval via the `user_dispatch_command` instruction.

- **Off-Chain Oracle**: For dynamic pricing, the program uses an off-chain oracle pattern. The service's backend signs payment details (price, command, timestamp), and the on-chain program verifies this signature before processing a payment. This keeps business logic flexible and off-chain, while keeping value transfer secure and on-chain.

- **Event-Driven Architecture**: The program emits detailed events for every significant action. Off-chain clients, like the `w3b2-solana-connector`, can listen for these events to synchronize state and trigger backend processes.

## On-Chain State Accounts

### `AdminProfile`

This PDA represents a service provider. Its address is derived from the admin's wallet key, ensuring one profile per admin.

- **PDA Seeds**: `[b"admin", authority.key().as_ref()]`
- **Key Fields**:
    - `authority`: The admin's wallet, with sole authority to manage the profile.
    - `oracle_authority`: The key trusted to sign price information for paid commands.
    - `balance`: The internal treasury where collected fees accumulate.
    - `unban_fee`: A configurable fee a user must pay to request an unban.

### `UserProfile`

This PDA represents a user's relationship with a specific service. Its address is derived from both the user's wallet and the admin's profile key, creating a unique link.

- **PDA Seeds**: `[b"user", authority.key().as_ref(), admin_profile.key().as_ref()]`
- **Key Fields**:
    - `authority`: The user's wallet, with sole authority to manage the profile.
    - `admin_profile_on_creation`: A permanent link to the `AdminProfile` this profile is associated with.
    - `deposit_balance`: The user's prepaid balance for this service.
    - `banned`: A flag indicating if the user is banned by the admin.

## Instruction Set

The program exposes a set of instructions for managing profiles and dispatching commands.

### Admin Instructions

| Instruction                | Description                                                                                             |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `admin_register_profile`   | Initializes a new `AdminProfile` PDA for a service provider.                                            |
| `admin_set_config`         | Updates configuration, such as the `oracle_authority`, `unban_fee`, or `communication_pubkey`.          |
| `admin_withdraw`           | Withdraws earned funds from the `AdminProfile`'s internal `balance`.                                    |
| `admin_ban_user`           | Bans a user, preventing them from using the service.                                                    |
| `admin_unban_user`         | Unbans a user, restoring their access.                                                                  |
| `admin_dispatch_command`   | Sends a non-financial command or notification to a user, emitting an on-chain event.                    |
| `admin_close_profile`      | Closes the `AdminProfile` account and refunds the rent lamports to the admin.                           |

### User Instructions

| Instruction                | Description                                                                                             |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `user_create_profile`      | Creates a `UserProfile` PDA, linking a user's wallet to a specific admin service.                       |
| `user_deposit`             | Deposits lamports into the `UserProfile`'s `deposit_balance` to pre-fund future payments.               |
| `user_withdraw`            | Withdraws unspent funds from the `deposit_balance`.                                                     |
| `user_update_comm_key`     | Updates the `communication_pubkey` for the user's profile.                                              |
| `user_request_unban`       | Allows a banned user to pay the `unban_fee` and request an unban from the admin.                        |
| `user_close_profile`       | Closes the `UserProfile` and refunds all remaining lamports (rent and deposit balance) to the user.     |

### Operational Instructions

| Instruction                | Description                                                                                             |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `user_dispatch_command`    | **Primary operational instruction.** A user calls a service's command, verifying a signed price from the admin's oracle and transferring payment if applicable. |
| `log_action`               | A generic instruction to log a significant off-chain action to the blockchain for an audit trail.       |

For more detailed information on each instruction's arguments, account requirements, and emitted events, please refer to the Rustdoc comments within the source code.

## Design Philosophy: The Unban Process

The process for unbanning a user is intentionally asynchronous and requires admin intervention. A user pays an `unban_fee` to request a review, and an off-chain listener notifies the admin, who must then manually approve the unban by calling `admin_unban_user`.

This "request for review" model was chosen over a simpler "pay-to-unban" atomic model for several key reasons:

1.  **Admin Sovereignty**: A ban is a disciplinary measure. If a user could automatically unban themselves, the ban would lose its meaning as a deterrent for malicious behavior (e.g., spam). The service administrator must have the final say.
2.  **Flexibility**: The current model allows the admin to unban a user for free (e.g., if the ban was an error) or to refuse an unban request even if the fee was paid.
3.  **Clear Separation of Concerns**: The smart contract's role is to act as an incorruptible financial arbiter. It verifiably records the facts: "Yes, the user paid the fee. Yes, they have requested a review." The final business decision remains off-chain, where it belongs.