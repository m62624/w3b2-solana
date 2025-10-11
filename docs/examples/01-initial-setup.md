# Example 1: Initial Setup & Wallet Creation

Before interacting with the toolset, you need a Solana wallet (keypair) and some SOL tokens on a local network.

## 1. Using the Solana CLI

The easiest way to create a wallet is with the Solana CLI, which is included in our `builder` Docker container.

First, start the builder service. This command will give you a shell inside the container.

```bash
# From the root of the project
docker compose run --rm builder bash
```

Inside the container, run the following command:

```bash
# This creates a new keypair file at the specified path
solana-keygen new --outfile /keys/my-wallet.json
```

The output will give you your public key and a seed phrase. **Save the seed phrase securely!**

```
pubkey: YOUR_PUBLIC_KEY
Save this seed phrase to recover your new keypair:
...
```

## 2. Airdropping Local SOL

To pay for transaction fees, you need SOL. Our local `solana-validator` service includes a faucet. From another terminal on your host machine, you can airdrop SOL to your new wallet.

First, find your public key:

```bash
# Run this inside the builder container if you're still in it
solana-keygen pubkey /keys/my-wallet.json
```

Then, use the `solana airdrop` command from your host machine, targeting the validator's RPC port (defined as `SOLANA_VALIDATOR_RPC_PORT` in your `.env` file, default is 8899).

```bash
# On your host machine
solana airdrop 10 YOUR_PUBLIC_KEY --url http://localhost:8899
```

You now have a funded wallet ready to interact with the on-chain program. The keypair file (`my-wallet.json`) is what you will use in client scripts to sign transactions.