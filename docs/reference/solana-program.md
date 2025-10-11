# Solana Program Reference

This document provides a reference for the on-chain program's instructions and accounts.

## Instructions

### `initialize`

Initializes a new main state account.

*   **Accounts:**
    *   `new_account`: The account to be initialized.
    *   `user`: The signer who will become the authority.
    *   `system_program`: The Solana System Program.
*   **Arguments:** None.

### `user_dispatch_command`

Executes a command after verifying an oracle's signature.

*   **Accounts:**
    *   `user`: The user executing the command.
    *   `target_account`: The state account to modify.
    *   `instructions`: The Instructions Sysvar, used to read the preceding signature verification instruction.
*   **Arguments:**
    *   `command: String`: The command to be executed.
*   **Pre-conditions:**
    *   This instruction **must** be immediately preceded by an `Ed25519Program.createInstructionWithPublicKey` instruction in the same transaction. The public key used in that instruction must match the oracle authority set in the `target_account`.