const { PublicKey } = require("@solana/web3.js");
const idl = require("./artifacts/w3b2_solana_program.json");

const CONFIG = {
  GATEWAY_HOST: process.env.GATEWAY_HOST || "localhost",
  GATEWAY_PORT: process.env.GATEWAY_PORT || "50051",
  SOLANA_RPC_URL: process.env.SOLANA_RPC_URL || "http://localhost:8899",
  PROGRAM_ID: new PublicKey(idl.address)
};

if (!CONFIG.PROGRAM_ID) throw new Error("IDL не содержит programId");

module.exports = { CONFIG, idl };