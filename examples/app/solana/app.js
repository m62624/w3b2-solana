const anchor = require("@coral-xyz/anchor");
const { Keypair, SystemProgram, PublicKey, LAMPORTS_PER_SOL } = require("@solana/web3.js");
const { Bot } = require("./bot");
const { sleep, airdrop } = require("./utils");
const { createGrpcClient, listenForEvents } = require("../grpc");
const { CONFIG, idl } = require("../config");

class SocialApp {
  constructor() {
    this.programId = CONFIG.PROGRAM_ID;
    this.connection = new anchor.web3.Connection(CONFIG.SOLANA_RPC_URL, "confirmed");
    this.admin = Keypair.generate();
    this.alice = Keypair.generate();
    this.oracle = Keypair.generate(); // 1. Создаем отдельный ключ для оракула
    this.bob = Keypair.generate();

    this.grpcClient = createGrpcClient();
    const adminProvider = new anchor.AnchorProvider(this.connection, new anchor.Wallet(this.admin), { preflightCommitment: "confirmed" });
    this.programAdmin = new anchor.Program(idl, adminProvider);

    [this.adminPda] = PublicKey.findProgramAddressSync([Buffer.from("admin"), this.admin.publicKey.toBuffer()], this.programId);
    [this.alicePda] = PublicKey.findProgramAddressSync([Buffer.from("user"), this.alice.publicKey.toBuffer(), this.adminPda.toBuffer()], this.programId);
    [this.bobPda] = PublicKey.findProgramAddressSync([Buffer.from("user"), this.bob.publicKey.toBuffer(), this.adminPda.toBuffer()], this.programId);

    this.botAlice = new Bot("Alice", this.alice, this.alicePda, this, this.oracle); // 2. Передаем ключ оракула в ботов
    this.botBob = new Bot("Bob", this.bob, this.bobPda, this, this.oracle);
  }

  getProvider(kp) {
    // This method is no longer needed with the new Program API.
    // We will modify getProgramForKeypair to handle this.
    throw new Error("getProvider is deprecated.");
  }

  getProgramForKeypair(kp) {
    const provider = new anchor.AnchorProvider(this.connection, new anchor.Wallet(kp), { preflightCommitment: "confirmed" });
    return new anchor.Program(idl, provider);
  }

  async setup() {
    await Promise.all([
      airdrop(this.connection, this.admin.publicKey, 2),
      airdrop(this.connection, this.alice.publicKey, 2),
      airdrop(this.connection, this.bob.publicKey, 2),
      airdrop(this.connection, this.oracle.publicKey, 1) // Даем немного SOL оракулу на всякий случай
    ]);

    console.log("Registering admin/user profiles...");
    await this.programAdmin.methods
      .adminRegisterProfile(this.admin.publicKey)
      .accounts({
        authority: this.admin.publicKey,
        adminProfile: this.adminPda,
        systemProgram: SystemProgram.programId
      })
      .rpc();

    // 3. Устанавливаем ключ оракула в профиле администратора
    console.log("Setting oracle authority...");
    await this.programAdmin.methods
      .adminSetConfig(this.oracle.publicKey, null, null, null)
      .accounts({
        authority: this.admin.publicKey,
        adminProfile: this.adminPda,
      })
      .rpc();

    const progAlice = this.getProgramForKeypair(this.alice);
    const progBob = this.getProgramForKeypair(this.bob);

    await Promise.all([
      progAlice.methods.userCreateProfile(this.adminPda, this.alice.publicKey)
        .accounts({
          authority: this.alice.publicKey,
          adminProfile: this.adminPda,
          userProfile: this.alicePda,
          systemProgram: SystemProgram.programId
        }).rpc(),

      progBob.methods.userCreateProfile(this.adminPda, this.bob.publicKey)
        .accounts({
          authority: this.bob.publicKey,
          adminProfile: this.adminPda,
          userProfile: this.bobPda,
          systemProgram: SystemProgram.programId
        }).rpc()
    ]);

    console.log("Setup complete.");
  }

  startListeners() {
    const pdas = { alice: this.alicePda, bob: this.bobPda };

    const handleEvent = (event) => {
      if (event.hasUserCommandDispatched()) {
        const e = event.getUserCommandDispatched();
        const sender = e.getSenderUserPda() === pdas.alice.toBase58() ? "Alice" : "Bob";
        const msg = Buffer.from(e.getPayload()).toString("utf8").replace(/^MSG:/, "");
        console.log(`[EVENT] ${sender}: ${msg}`);
      }
    };

    listenForEvents(this.grpcClient, "Alice", this.alicePda, { onData: handleEvent });
    listenForEvents(this.grpcClient, "Bob", this.bobPda, { onData: handleEvent });
  }

  async startChat() {
    let turn = 0;
    while (true) {
      const [s, r] = turn % 2 === 0 ? [this.botAlice, this.botBob] : [this.botBob, this.botAlice];
      await s.sendMessage(r.name, `Hello! This is message #${s.messageCount + 1}`);
      turn++;
      await sleep(5000);
    }
  }

  async start() {
    await this.setup();
    this.startListeners();
    await this.startChat();
  }
}

async function main() {
  try {
    console.log("Initializing Social App...");
    const app = new SocialApp();
    console.log("Starting Social App simulation...");
    await app.start();
  } catch (err) {
    console.error("An error occurred during the simulation:", err);
    process.exit(1);
  }
}

main();