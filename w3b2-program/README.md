# W3B2 On-Chain Program (`w3b2-program`)

This is the on-chain program for the W3B2 protocol. It provides the core logic for creating and managing user and service provider profiles, handling financial interactions, and facilitating a secure, auditable communication channel between off-chain parties.

## Core Concept

The program implements a **"Per-Service Profile"** model. A single user wallet (`authority`) can create multiple on-chain profiles (PDAs). Each profile is unique to a specific service (an "Admin") the user interacts with.

This architecture provides:

  * **Fund Isolation:** A user's deposit for one service is completely separate from their deposit for another.
  * **Non-Custodial Payments:** The user's main wallet signs for deposits, but the funds used for day-to-day service payments are held in program-controlled PDA accounts.

## Core On-Chain Entities

The program uses two primary Program Derived Addresses (PDAs) to manage state:

### `AdminProfile` PDA

  * **Represents:** A service provider (an "Admin").
  * **Stores:** The admin's wallet key (`authority`), a `communication_pubkey`, a dynamic `prices` list, and its earned `balance`.
  * **PDA Seeds:** `[b"admin", admin_authority.key().as_ref()]`

### `UserProfile` PDA

  * **Represents:** A user's relationship with and financial deposit for a *specific* Admin service.
  * **Stores:** The user's wallet key (`authority`), a `communication_pubkey`, the `admin_profile_on_creation` (the PDA key of the admin service) it's linked to, and the user's `deposit_balance`.
  * **PDA Seeds:** `[b"user", user_authority.key().as_ref(), admin_profile_pda.key().as_ref()]`

## Instruction Interface

All state-changing instructions require a signature from the appropriate wallet (`authority`).

### Admin Instructions

| Instruction              | Signer                       | Arguments                      | Description                                                               |
| ------------------------ | ---------------------------- | ------------------------------ | ------------------------------------------------------------------------- |
| `admin_register_profile` | Admin's wallet (`authority`) | `communication_pubkey: Pubkey` | Creates the `AdminProfile` PDA.                                           |
| `admin_update_comm_key`  | Admin's wallet (`authority`) | `new_key: Pubkey`              | Updates the admin's communication public key.                             |
| `admin_update_prices`    | Admin's wallet (`authority`) | `new_prices: Vec<PriceEntry>`  | Updates the service price list, reallocating the PDA to fit the new size. |
| `admin_withdraw`         | Admin's wallet (`authority`) | `amount: u64`                  | Withdraws earned funds from the `AdminProfile`'s internal balance.        |
| `admin_close_profile`    | Admin's wallet (`authority`) | -                              | Closes the `AdminProfile` and refunds its rent lamports.                  |

### User Instructions

| Instruction            | Signer                      | Arguments                                              | Description                                                                     |
| ---------------------- | --------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------- |
| `user_create_profile`  | User's wallet (`authority`) | `target_admin: Pubkey`, `communication_pubkey: Pubkey` | Creates a `UserProfile` PDA, linking the user to a specific admin service.      |
| `user_update_comm_key` | User's wallet (`authority`) | `new_key: Pubkey`                                      | Updates the user's communication public key for a specific service profile.     |
| `user_deposit`         | User's wallet (`authority`) | `amount: u64`                                          | Deposits lamports into the `UserProfile` PDA.                                   |
| `user_withdraw`        | User's wallet (`authority`) | `amount: u64`                                          | Withdraws unspent funds from the `UserProfile`'s deposit balance.               |
| `user_close_profile`   | User's wallet (`authority`) | -                                                      | Closes the `UserProfile` and refunds all lamports (deposit + rent) to the user. |

### Operational Instructions

| Instruction              | Signer                       | Arguments                                               | Description                                                                                                                     |
| ------------------------ | ---------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `user_dispatch_command`  | User's wallet (`authority`)  | `command_id: u16`, `payload: Vec<u8>`                   | A user calls a service's API. If the command has a price, funds are transferred from the user's deposit to the admin's balance. |
| `admin_dispatch_command` | Admin's wallet (`authority`) | `command_id: u64`, `payload: Vec<u8>`                   | An admin sends a command/notification to a user. This is a non-financial transaction used to emit an event.                     |
| `log_action`             | User or Admin wallet         | `target: Pubkey`, `session_id: u64`, `action_code: u16` | A generic instruction to log a significant off-chain action to the blockchain for auditing purposes.                            |

## Off-Chain Interaction

The program interacts with off-chain components (like `w3b2-gateway` via `w3b2-connector`) through two mechanisms:

### 1. Solana Events

Every instruction emits a corresponding event (e.g., `AdminProfileRegistered`, `UserCommandDispatched`). These events create an immutable, auditable log of all activity within the protocol.

### 2. Opaque `payload` Field

The `user_dispatch_command` and `admin_dispatch_command` instructions include an opaque `payload: Vec<u8>` field. This field is not interpreted on-chain. Instead, it acts as a message bus, allowing off-chain components to define and use their own serialization formats (like Protobuf or JSON) to pass complex data structures through the blockchain. The `protocols.rs` file contains examples of such serializable structs.

## API Definition (Protobuf)

The API contract, including all on-chain events and the gRPC service definition, is formally defined using **Protocol Buffers (Protobuf)** in the `proto/` directory of the root project.

- `types.proto`: Defines all message structures.
- `gateway.proto`: Defines the `BridgeGatewayService` RPC methods.