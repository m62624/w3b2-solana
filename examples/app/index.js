const { SocialApp } = require("./solana/app");

(async () => {
  try {
    console.log("--- Starting Social App ---");
    const app = new SocialApp();
    await app.start();
  } catch (e) {
    console.error("FATAL:", e);
    process.exit(1);
  }
})();