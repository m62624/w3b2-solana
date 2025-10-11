# Example 2: The Ed25519 Pre-Instruction

The core of our oracle pattern is the `Ed25519Program.createInstructionWithPublicKey` instruction. It allows us to verify a signature from an off-chain authority (our Gateway) within an on-chain program.

This instruction doesn't perform an action itself; it simply loads signature data into the transaction's context. The subsequent instruction (our `user_dispatch_command`) then reads this data to confirm its authenticity.

## How It Works

The instruction requires four pieces of data:

1.  `publicKey`: The public key of the oracle that signed the message. This **must** be the raw byte array (`Buffer` or `Uint8Array`), not a `PublicKey` object.
2.  `message`: The exact, byte-for-byte message that the oracle signed.
3.  `signature`: The Ed25519 signature produced by the oracle.
4.  `instructionIndex` (Optional): The index of the instruction within the transaction. It's safest to omit this and simply ensure the instruction is placed correctly.

## TypeScript Client Example

Here is a focused example of creating *only* this instruction.

```typescript
import { Ed25519Program, PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer'; // Ensure buffer is available

// --- Data you would get from your setup and the Gateway ---

// 1. The Oracle's public key. This is configured in your program
//    and should be a constant in your client.
const ORACLE_PUBLIC_KEY_STRING = "YOUR_ORACLE_PUBLIC_KEY"; // Replace with actual key
const oraclePublicKeyBytes = new PublicKey(ORACLE_PUBLIC_KEY_STRING).toBytes();

// 2. The message that was signed. This is the data your client
//    sent to the gateway.
const message = Buffer.from("Data that was signed by the oracle");

// 3. The signature returned by the gateway after it verified your payment.
const signatureBase64 = "BASE64_SIGNATURE_FROM_GATEWAY"; // Replace with actual signature
const signatureBytes = Buffer.from(signatureBase64, 'base64');


// --- Creating the Instruction ---

const verifyInstruction = Ed25519Program.createInstructionWithPublicKey({
    publicKey: oraclePublicKeyBytes,
    message: message,
    signature: signatureBytes,
});

// This `verifyInstruction` is now ready to be added to a transaction,
// immediately before your main program instruction.
```

### Important Considerations

*   **Order is Critical:** The `Ed25519Program` instruction **must** come immediately before the instruction that needs to verify it within the same transaction.
*   **Data Must Match Exactly:** The `message` bytes in this instruction must be identical to the bytes the on-chain program reconstructs. Any difference, even a single byte, will cause the signature verification to fail.
*   **Network Consistency:** The entire interaction must occur on the same network. The Gateway should connect to the same RPC endpoint (e.g., `http://solana-validator:8899` inside Docker) that your client script targets (`http://localhost:8899` from the host). The `.env` file should be the source of truth for these URLs.