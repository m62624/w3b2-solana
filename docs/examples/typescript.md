# TypeScript Example

This example shows how to build a full-stack Web application that interacts with the W3B2 protocol, featuring a Node.js oracle backend and a browser-based client.

## Prerequisites

-   Node.js and npm/yarn.
-   `@grpc/grpc-js` and `@grpc/proto-loader` for the gRPC client.
-   `@solana/web3.js` for Solana interactions.
-   `@solana/wallet-adapter-base` and a specific wallet adapter (e.g., `@solana/wallet-adapter-react`) for signing in the browser.
-   `tweetnacl` for signing on the backend oracle.

First, you'll need to generate the JavaScript gRPC client from the `.proto` files. Tools like `grpc-tools` can automate this.

## 1. The Oracle Service (Your Node.js/Express Backend)

This is the backend service you control. It securely stores the oracle's private key and signs payment quotes for your users.

```typescript
import express from 'express';
import { Keypair } from '@solana/web3.js';
import nacl from 'tweetnacl';
import { Buffer } from 'buffer';

const app = express();
app.use(express.json());

// In a real app, load this from a secure vault (e.g., AWS KMS, HashiCorp Vault)
// or from environment variables. DO NOT hardcode private keys.
const ORACLE_SECRET_KEY = Buffer.from([...]); // Your 64-byte secret key
const oracleKeypair = Keypair.fromSecretKey(ORACLE_SECRET_KEY);

// This endpoint creates the signed quote for the client
app.post('/api/quote', (req, res) => {
    const { commandId } = req.body;

    const price = 50000; // Look up price for the command
    const timestamp = Math.floor(Date.now() / 1000);

    // Construct the message buffer (as little-endian)
    const commandIdBuf = Buffer.alloc(2);
    commandIdBuf.writeUInt16LE(commandId, 0);

    const priceBuf = Buffer.alloc(8);
    priceBuf.writeBigUInt64LE(BigInt(price), 0);

    const timestampBuf = Buffer.alloc(8);
    timestampBuf.writeBigUInt64LE(BigInt(timestamp), 0);

    const message = Buffer.concat([commandIdBuf, priceBuf, timestampBuf]);

    // Sign the message with the oracle's private key
    const signature = nacl.sign.detached(message, oracleKeypair.secretKey);

    res.json({
        commandId,
        price: price.toString(),
        timestamp: timestamp.toString(),
        signature: Buffer.from(signature).toString('base64'),
        oraclePubkey: oracleKeypair.publicKey.toBase58(),
    });
});

app.listen(3001, () => console.log('Oracle service running on port 3001'));
```

## 2. The Client (Browser-Side Logic)

This code would run in your React/Vue/Svelte application. It orchestrates the flow: fetch quote -> prepare tx -> sign -> submit.

```typescript
import { Connection, Transaction } from '@solana/web3.js';
import { useWallet, WalletContextState } from '@solana/wallet-adapter-react';
import { BridgeGatewayServiceClient } from './generated/gateway_grpc_pb'; // Your generated client
import { PrepareUserDispatchCommandRequest, SubmitTransactionRequest } from './generated/gateway_pb';
import { Buffer } from 'buffer';

// The main function to be called when the user clicks "Generate"
async function handleGenerateImage(wallet: WalletContextState) {
    if (!wallet.publicKey || !wallet.signTransaction) {
        throw new Error('Wallet not connected or does not support signing');
    }

    // --- Step 1: Get quote from your oracle ---
    const quoteResponse = await fetch('http://localhost:3001/api/quote', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ commandId: 42 }),
    });
    const quote = await quoteResponse.json();

    // --- Step 2: Prepare transaction via W3B2 Gateway ---
    const gatewayClient = new BridgeGatewayServiceClient('http://localhost:50051', ...);
    const userProfilePda = '...'; // The user's profile PDA for this service

    const prepareReq = new PrepareUserDispatchCommandRequest();
    prepareReq.setUserProfilePda(userProfilePda);
    prepareReq.setUserAuthority(wallet.publicKey.toBase58());
    prepareReq.setOracleAuthority(quote.oraclePubkey);
    prepareReq.setCommandId(quote.commandId);
    prepareReq.setPrice(quote.price);
    prepareReq.setTimestamp(quote.timestamp);
    prepareReq.setSignature(Buffer.from(quote.signature, 'base64'));

    const unsignedTxRes = await gatewayClient.prepareUserDispatchCommand(prepareReq);
    const txBytes = unsignedTxRes.getTransaction_asU8();

    // --- Step 3: Sign the transaction using the user's wallet ---
    const transaction = Transaction.from(txBytes);
    // The gateway does not set the blockhash, the client must.
    const connection = new Connection('http://localhost:8899');
    transaction.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    transaction.feePayer = wallet.publicKey;

    const signedTx = await wallet.signTransaction(transaction);

    // --- Step 4: Submit the signed transaction ---
    const submitReq = new SubmitTransactionRequest();
    submitReq.setSignedTransaction(signedTx.serialize());

    const submitRes = await gatewayClient.submitTransaction(submitReq);

    console.log('Transaction successful! Signature:', submitRes.getSignature());
    return submitRes.getSignature();
}
```

## 3. Listening for Events (Node.js Backend)

Your backend service should listen for `UserCommandDispatched` events to know when to actually perform the requested work.

```typescript
import { ListenRequest } from './generated/gateway_pb';

function listenForServiceEvents(adminProfilePda: string) {
    const gatewayClient = new BridgeGatewayServiceClient('http://localhost:50051', ...);

    const listenReq = new ListenRequest();
    listenReq.setPda(adminProfilePda);

    const stream = gatewayClient.listenAsAdmin(listenReq);

    console.log(`Listening for events on ${adminProfilePda}...`);

    stream.on('data', (item) => {
        const event = item.getEvent();
        if (event.hasUserCommandDispatched()) {
            const dispatchEvent = event.getUserCommandDispatched();
            console.log(`[SUCCESS] User ${dispatchEvent.getUserAuthority()} dispatched command ${dispatchEvent.getCommandId()}`);
            // TODO: Kick off the actual business logic (e.g., generate the AI image)
        }
    });

    stream.on('error', (err) => console.error('Stream error:', err));
    stream.on('end', () => console.log('Stream ended.'));
}
```