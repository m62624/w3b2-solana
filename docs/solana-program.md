# On-Chain Program Reference

The core on-chain smart contract for the W3B2 toolset.

This Anchor program provides a secure and verifiable framework for Web2 services to
interact with the Solana blockchain. It enables services to manage user profiles,
handle payments, and dispatch commands in a non-custodial manner, where users
always retain control of their funds and sign all transactions with their own wallets.

## Core Concepts

- **Admin & User Profiles:** The program establishes two primary PDA account types:
  - `AdminProfile`: Represents the service provider (the "Admin"). It holds configuration
    like the oracle key and serves as a treasury for collected fees.
  - `UserProfile`: Represents an end-user's relationship with a specific service. It
    holds the user's pre-paid deposit balance for that service.

- **Non-Custodial Payments:** Users deposit funds into their own `UserProfile` PDA, which
  is controlled by the program. Payments for services are transferred from the user's
  profile to the admin's profile only upon the user's explicit, signed approval via the
  `user_dispatch_command` instruction. This means users always retain control of their funds.

- **Off-Chain Oracle:** For dynamic pricing, the program uses an off-chain oracle pattern.
  The service's backend signs payment details (price, command, timestamp), and the on-chain
  program verifies this signature before processing a payment. This keeps business logic
  flexible and off-chain, while keeping value transfer secure and on-chain.

- **Event-Driven Architecture:** The program emits detailed events for every significant
  action (e.g., `UserProfileCreated`, `UserCommandDispatched`). Off-chain clients, like the
  `w3b2-solana-connector`, can listen for these events to synchronize state and trigger backend
  processes.

## Instructions

The program exposes a comprehensive set of instructions for managing the service and user interactions.

### Admin Instructions
*   `admin_register_profile`: Creates an `AdminProfile` for a new service.
*   `admin_set_config`: Updates settings like the oracle key or unban fee.
*   `admin_withdraw`: Withdraws earned funds from the admin's internal balance.
*   `admin_ban_user`: Bans a user.
*   `admin_unban_user`: Unbans a user.
*   `admin_dispatch_command`: Sends a non-financial command/notification to a user.
*   `admin_close_profile`: Closes the admin profile and reclaims rent.

### User Instructions
*   `user_create_profile`: Creates a `UserProfile` linked to an admin.
*   `user_update_comm_key`: Updates the user's communication key.
*   `user_deposit`: Deposits lamports into the user's on-chain profile.
*   `user_withdraw`: Withdraws lamports from the user's deposit balance.
*   `user_request_unban`: Pays a fee to request an unban.
*   `user_dispatch_command`: Executes a paid command by verifying an oracle signature.
*   `user_close_profile`: Closes the user profile and reclaims all lamports.

For detailed arguments and account requirements for each instruction, please refer to the IDL in `artifacts/w3b2_solana_program.json` or the source code in `w3b2-solana-program/src/lib.rs`.