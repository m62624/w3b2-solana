# TypeScript Client Reference

The TypeScript client provides helper functions to interact with the Gateway and the Solana Program.

## Functions

### `getCommandMessage(user: PublicKey, targetAccount: PublicKey, command: string): Buffer`

Creates the standardized message buffer that the oracle needs to sign. This buffer is what the on-chain program will re-create and verify against the provided signature.

*   **Arguments:**
    *   `user`: The public key of the user's wallet.
    *   `targetAccount`: The public key of the on-chain state account.
    *   `command`: The string command to be executed.
*   **Returns:** A `Buffer` containing the message data.

### `createDispatchTransaction(program: Program, user: PublicKey, targetAccount: PublicKey, command: string, oracleSignature: Buffer): Transaction`

Constructs the full Solana transaction, including the required signature-verification pre-instruction and the main `userDispatchCommand` instruction.

*   **Arguments:**
    *   `program`: An initialized Anchor program instance.
    *   `user`: The public key of the user's wallet.
    *   `targetAccount`: The public key of the on-chain state account.
    *   `command`: The string command to be executed.
    *   `oracleSignature`: The raw signature `Buffer` returned from the gateway.
*   **Returns:** A Solana `Transaction` object, ready to be signed and sent.