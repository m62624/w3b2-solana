const { LAMPORTS_PER_SOL } = require("@solana/web3.js");

const sleep = (ms) => new Promise(r => setTimeout(r, ms));

async function airdrop(conn, pubkey, sol) {
  const lamports = sol * LAMPORTS_PER_SOL;
  try {
    const sig = await conn.requestAirdrop(pubkey, lamports);
    await conn.confirmTransaction(sig, "confirmed");
    console.log(`Airdropped ${sol} SOL → ${pubkey.toBase58()}`);
  } catch (e) {
    console.error(`Airdrop failed: ${e.message}`);
  }
}

module.exports = { sleep, airdrop };