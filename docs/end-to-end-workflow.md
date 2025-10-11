# Tutorial: End-to-End Workflow

This tutorial demonstrates the primary use case: executing a paid command on-chain.

**Goal:** A user will pay the off-chain Gateway, and in return, the Gateway will provide a signature that authorizes the `user_dispatch_command` instruction on the Solana program.

### The Flow

1.  **Client:** Prepare the command data.
2.  **Client -> Gateway:** Request a signature for this data. The Gateway will handle payment logic internally.
3.  **Gateway -> Client:** Return an oracle signature if payment is successful.
4.  **Client:** Construct a transaction including the oracle's signature and the command instruction.
5.  **Client -> Solana:** Send the transaction to the network for execution.
6.  **Solana Program:** Verify the oracle signature and execute the command.

---

### Step 1: Prepare the Command (TypeScript Client)

First, the client defines what it wants to do. The command is a simple string in this example. The client also specifies which on-chain account this command is for.

```typescript
// src/client.ts
import { PublicKey } from '@solana/web3.js';
import { getCommandMessage } from './utils';

const userWalletPubkey = new PublicKey("USER_WALLET_ADDRESS");
const targetAccountPubkey = new PublicKey("ON_CHAIN_ACCOUNT_ADDRESS");
const command = "EXECUTE_PREMIUM_ACTION";

// This function creates a standardized message buffer for the oracle to sign.
// It must match the format expected by the on-chain program.
const messageToSign = getCommandMessage(
  userWalletPubkey,
  targetAccountPubkey,
  command
);
```

### Step 2: Request Oracle Signature (TypeScript Client & Gateway API)

The client sends the message to the Gateway's `/sign_command` endpoint. The Gateway is responsible for authenticating the user and verifying they have paid for this action.

```typescript
// src/client.ts
import axios from 'axios';

// The Gateway API requires the message to be sent in base64 format.
const messageBase64 = Buffer.from(messageToSign).toString('base64');

let oracleSignature: string;

try {
  const response = await axios.post('http://localhost:8000/api/v1/sign_command', {
    message: messageBase64,
  }, {
    headers: { 'Authorization': 'Bearer USER_AUTH_TOKEN' } // The Gateway handles user auth
  });

  // The Gateway returns the signature, also in base64.
  oracleSignature = response.data.signature;

} catch (error) {
  console.error("Failed to get oracle signature:", error.response.data);
  throw error;
}
```

### Step 3: Construct the Full Transaction (TypeScript Client)

This is the most critical step. A `user_dispatch_command` instruction **must** be preceded by an `Ed25519Program.createInstructionWithPublicKey` instruction in the same transaction. This "pre-instruction" loads the oracle's signature into the runtime for the main program to use.

```typescript
// src/client.ts
import {
  Transaction,
  sendAndConfirmTransaction,
  Ed25519Program,
} from '@solana/web3.js';
import { MyProgram } from './program'; // Your Anchor-generated client

const program = new MyProgram(...); // Initialize your program client
const oracleAuthorityPubkey = new PublicKey("ORACLE_PUBLIC_KEY_CONFIGURED_ON_CHAIN");

// Pre-instruction: Load the oracle signature for verification.
const verifyInstruction = Ed25519Program.createInstructionWithPublicKey({
  publicKey: oracleAuthorityPubkey.toBytes(),
  message: messageToSign,
  signature: Buffer.from(oracleSignature, 'base64'),
});

// Main instruction: The actual command to execute.
const dispatchInstruction = await program.methods
  .userDispatchCommand(command)
  .accounts({
    user: userWalletPubkey,
    targetAccount: targetAccountPubkey,
    // The instructions sysvar is how the program reads the signature
    // from the preceding instruction.
    instructions: web3.SYSVAR_INSTRUCTIONS_PUBKEY,
  })
  .instruction();

// Build the transaction
const transaction = new Transaction()
  .add(verifyInstruction) // MUST BE FIRST!
  .add(dispatchInstruction);

// Sign and send
const signature = await sendAndConfirmTransaction(connection, transaction, [userWallet]);
console.log("Transaction confirmed:", signature);
```

### Step 4: On-Chain Verification (Rust Solana Program)

You don't need to write this code, but it's important to understand how it works. The on-chain program verifies the work done by the client and gateway.

```rust
// programs/w3b2-solana-program/src/lib.rs

// ...
use solana_program::sysvar::instructions::{load_current_index_checked, load_instruction_at_checked};

#[program]
pub mod w3b2_solana_program {
    pub fn user_dispatch_command(ctx: Context<UserDispatchCommand>, command: String) -> Result<()> {
        // 1. Get the preceding instruction from the Instructions Sysvar
        let ixs = ctx.accounts.instructions.to_account_info();
        let current_index = load_current_index_checked(&ixs)? as usize;
        let verify_ix = load_instruction_at_checked(current_index - 1, &ixs)?;

        // 2. Verify it's the correct Ed25519 signature verification instruction
        // (Code to check program_id, message data, and public key is here)
        // ...

        // 3. If verification passes, execute the command logic
        msg!("Oracle signature verified. Executing command: {}", command);
        // ... update state, etc.

        Ok(())
    }
}

#[derive(Accounts)]
pub struct UserDispatchCommand<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub target_account: Account<'info, MyState>,
    /// CHECK: This is not dangerous because we only read from it.
    #[account(address = sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}
```

This completes the full, secure workflow, documented with code examples for each component of the system.