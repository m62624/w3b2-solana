# System Architecture

This document describes the core architectural patterns of the W3B2 toolset, focusing on the non-custodial account model and the use of a developer-owned oracle for transaction validation.

## 1. Non-Custodial Account Model

The on-chain program uses a non-custodial design to ensure that users retain control over their funds. This is achieved through a specific structure of Program Derived Addresses (PDAs) and ownership rules.

-   **User-Controlled Deposits**: Funds are held in a `UserProfile` PDA. The authority to deposit funds into this account or withdraw the entire remaining balance is restricted to the user's wallet.
-   **Program-Controlled Debits**: The service provider (the "Admin") cannot directly access funds in a `UserProfile`. The on-chain program only permits the transfer of funds from a user's profile to the admin's profile under specific, predefined conditions—namely, when the user signs a `user_dispatch_command` transaction for a service with a known price.
-   **On-Chain Transparency**: All financial movements are standard Solana transactions, making them publicly verifiable.

This model removes the need for users to trust the service provider with their funds, as control is enforced by the on-chain program's logic.

## 2. Developer-Owned Oracle Pattern

The system is designed to keep a service's business logic (e.g., pricing, access control) off-chain. To connect this off-chain logic to the on-chain program securely, the system uses a "developer-owned oracle" pattern.

**The service developer is responsible for running an oracle service.** This service's function is to sign data related to a user's action, creating a "quote" that the on-chain program can verify. The W3B2 components (gateway, connector) do not create or manage oracle keys; they only package the signature for on-chain verification.

### End-to-End Signature and Transaction Flow

The following sequence details how a paid user command is processed:

**1. Oracle Service Prepares and Signs Data**

When a user initiates an action, the developer's backend service (the oracle) assembles the relevant data and signs it.

1.  **Data construction**: The service constructs a message containing the details of the action, such as a `command_id`, the `price`, and a `timestamp` (to prevent replay attacks).
2.  **Signing**: The service signs this message with its private oracle key. This key must be kept secure and is never exposed to the client or the W3B2 components.
3.  **Payload generation**: The service returns a payload (e.g., JSON) to the user's client containing the signed data and the public key of the oracle for verification.

```json
{
  "command_id": 123,
  "price": 100000,
  "timestamp": 1678886400,
  "signature": "base64_encoded_signature...",
  "oracle_pubkey": "Pubkey_of_the_oracle..."
}
```

**2. Client Prepares the On-Chain Transaction**

The user's client (e.g., a web browser) uses the gateway or connector to prepare the final transaction.

1.  The client calls the `PrepareUserDispatchCommand` method, passing the data from the oracle's response.
2.  The gateway/connector constructs a Solana transaction containing two essential instructions:
    -   An `Ed25519` instruction to verify the `signature` from the payload.
    -   The `user_dispatch_command` instruction, which contains the command details.
3.  The gateway/connector returns the **unsigned** transaction to the client.

**3. Client Signs and Submits**

1.  The user signs the transaction with their wallet. This signature authorizes the potential transfer of funds from their `UserProfile`.
2.  The client submits the now-signed transaction to the Solana network (typically via the gateway's `SubmitTransaction` endpoint).

The on-chain program executes the transaction only if both the oracle's signature and the user's signature are valid. This separation of concerns ensures that the developer retains full control over their business logic (via the oracle) while the user retains full control over their funds. The absence of signature creation methods in the connector and gateway is a deliberate security feature of this architecture.

## 3. User Management: Banning and Unbanning

The on-chain program provides a mechanism for service providers (Admins) to manage user access by banning and unbanning them. This functionality is intended for moderation and to prevent abuse of the service.

### Banning a User

An Admin can ban any user who has created a `UserProfile` for their service. The `admin_ban_user` instruction sets a `banned` flag in the user's on-chain profile. When this flag is `true`, the user is blocked from performing most actions, including:

*   Dispatching commands (`user_dispatch_command`)
*   Withdrawing funds (`user_withdraw`)
*   Updating their communication key (`user_update_comm_key`)
*   Closing their profile (`user_close_profile`)

The ban is enforced by the on-chain program, ensuring that a banned user cannot interact with the service's core functions.

### Unbanning a User

An Admin can lift a ban at any time using the `admin_unban_user` instruction. This resets the `banned` flag to `false`, restoring the user's access.

### Requesting an Unban (User-Initiated)

A banned user has one available action: to request an unban. This is a paid operation, designed to create a hurdle for spam or abuse while providing a path for legitimate users to appeal.

1.  **Configurable Fee**: The Admin can set an "unban fee" using the `admin_set_config` instruction. This fee is stored in the `AdminProfile`.
2.  **User Request**: The user calls the `user_request_unban` instruction. This action verifies that the user has sufficient funds in their `UserProfile` deposit balance to pay the fee.
3.  **Fee Transfer**: The on-chain program transfers the `unban_fee` amount from the user's deposit balance to the admin's internal balance.
4.  **Flag Set**: The instruction sets the `unban_requested` flag in the user's profile to `true`.

This process does **not** automatically unban the user. It serves as a formal, on-chain request that the Admin can review. The Admin must still manually call `admin_unban_user` to restore the user's access. This two-step process ensures the Admin retains final control over access to their service.