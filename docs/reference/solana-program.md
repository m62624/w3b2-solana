# On-Chain Program Reference

This document provides a detailed reference for all instructions available in the W3B2 Solana program. The program acts as the ultimate source of truth and financial arbiter for the entire system.

For details on required accounts, arguments, and error codes, developers should refer to the program's IDL (`w3b2_solana_program.json`) or the source code in the `w3b2-solana-program` crate.

## Admin Instructions

These instructions are callable only by the designated `authority` of an `AdminProfile`. They are used for managing the service's on-chain presence.

---

### `admin_register_profile`
Initializes a new `AdminProfile` PDA for a service provider. This creates the on-chain representation of a service, setting its owner (`authority`) and its off-chain communication key. The oracle authority is set to the admin's own key by default but can be changed later.

**Emits:** `AdminProfileRegistered`

---

### `admin_close_profile`
Closes an `AdminProfile` account and refunds its rent lamports to the owner.
**Note:** This only returns rent lamports. Any funds in the internal `balance` must be withdrawn via `admin_withdraw` first.

**Emits:** `AdminProfileClosed`

---

### `admin_set_config`
Sets or updates the configuration for an existing `AdminProfile`. Allows the admin to update the `oracle_authority`, `timestamp_validity_seconds`, `communication_pubkey`, and `unban_fee`. Any field passed as `None` is ignored.

**Emits:** `AdminConfigUpdated`, `AdminUnbanFeeUpdated` (if fee changes)

---

### `admin_withdraw`
Withdraws earned funds from an `AdminProfile`'s internal `balance`. Performs a lamport transfer from the `AdminProfile` PDA to a specified destination account.

**Emits:** `AdminFundsWithdrawn`

---

### `admin_dispatch_command`
Dispatches a non-financial command or notification from an admin to a user. Its primary purpose is to emit an `AdminCommandDispatched` event that an off-chain user connector can listen to.

**Emits:** `AdminCommandDispatched`

---

### `admin_ban_user`
Bans a user by setting the `banned` flag on their `UserProfile` to `true`. This prevents the user from calling `user_dispatch_command`.

**Emits:** `UserBanned`

---

### `admin_unban_user`
Unbans a user by setting the `banned` flag to `false`. This is a discretionary action. See the "Request for Review" model in [Core Concepts](../architecture/concepts.md) for the design philosophy.

**Emits:** `UserUnbanned`

## User Instructions

These instructions are callable by end-users to manage their profile and interact with a service.

---

### `user_create_profile`
Creates a `UserProfile` PDA, linking a user's wallet to a specific admin service. This creates a permanent, verifiable link between the user and the service.

**Emits:** `UserProfileCreated`

---

### `user_update_comm_key`
Updates the `communication_pubkey` for an existing `UserProfile`.

**Emits:** `UserCommKeyUpdated`

---

### `user_close_profile`
Closes a `UserProfile` account. All lamports held by the PDA (both for rent and from any remaining `deposit_balance`) are safely returned to the user's wallet.

**Emits:** `UserProfileClosed`

---

### `user_deposit`
Deposits lamports into a `UserProfile` PDA via a CPI to the System Program. This pre-funds a user's account for future payments.

**Emits:** `UserFundsDeposited`

---

### `user_withdraw`
Withdraws unspent funds from a `UserProfile`'s `deposit_balance`. Performs a direct lamport transfer from the PDA to a destination account.

**Emits:** `UserFundsWithdrawn`

---

### `user_request_unban`
Allows a banned user to pay a fee (if set by the admin) to request an unban. This sets the `unban_requested` flag to `true` and emits an event. It does **not** automatically unban the user.

**Emits:** `UserUnbanRequested`

## Operational Instructions

These instructions are central to the service's operation.

---

### `user_dispatch_command`
The primary instruction for user-service interaction. It dispatches a command from a user, verifying a price signature from the admin's designated oracle. If the price is non-zero, it transfers payment from the user's profile to the admin's profile.

**Pre-requisite:** This instruction **must** be preceded by an `ed25519` signature verification instruction in the same transaction.

**Emits:** `UserCommandDispatched`

---

### `log_action`
Logs a significant off-chain action to the blockchain for an immutable audit trail. Can be signed by either the user or the admin.

**Emits:** `OffChainActionLogged`