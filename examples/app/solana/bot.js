const anchor = require("@coral-xyz/anchor");
const { Keypair, SYSVAR_INSTRUCTIONS_PUBKEY } = require("@solana/web3.js");
const nacl = require("tweetnacl");

class Bot {
  constructor(name, keypair, pda, app, oracleKeypair) {
    this.name = name;
    this.keypair = keypair;
    this.pda = pda;
    this.app = app;
    this.oracle = oracleKeypair; // Сохраняем правильный ключ оракула
    this.messageCount = 0;
    this.program = app.getProgramForKeypair(keypair);
  }

  async sendMessage(toName, text) {
    console.log(`[${this.name}] → [${toName}]: ${text}`);
    const payload = Buffer.from(`MSG:${text}`);

    const commandId = 1;
    const price = new anchor.BN(0);
    const ts = new anchor.BN(Math.floor(Date.now() / 1000));

    const msg = Buffer.concat([
      Buffer.from(Uint16Array.of(commandId).buffer),
      Buffer.from(price.toArray("le", 8)),
      Buffer.from(ts.toArray("le", 8))
    ]);

    const sig = nacl.sign.detached(msg, this.oracle.secretKey); // Используем правильный ключ

    await this.program.methods
      .userDispatchCommand(commandId, price, ts, payload)
      .accounts({
        authority: this.keypair.publicKey,
        userProfile: this.pda,
        adminProfile: this.app.adminPda,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .preInstructions([
        anchor.web3.Ed25519Program.createInstructionWithPublicKey({
          publicKey: this.oracle.publicKey.toBytes(), // Используем правильный ключ
          message: msg,
          signature: sig,
        })
      ])
      .rpc();

    this.messageCount++;
  }
}

module.exports = { Bot };