const grpc = require("@grpc/grpc-js");
const { BridgeGatewayServiceClient } = require("./gateway_grpc_pb");
const { ListenRequest } = require("./types_pb");
const { CONFIG } = require("./config");

function createGrpcClient() {
  const addr = `${CONFIG.GATEWAY_HOST}:${CONFIG.GATEWAY_PORT}`;
  console.log(`[gRPC] Connecting to ${addr}`);
  return new BridgeGatewayServiceClient(addr, grpc.credentials.createInsecure());
}

function listenForEvents(client, name, pda, handlers) {
  const req = new ListenRequest();
  req.setPda(pda.toBase58());
  console.log(`[${name}] Listening for events on ${pda.toBase58()}`);

  const stream = client.listenAsUser(req);

  stream.on("data", (item) => handlers.onData(item.getEvent()));
  stream.on("error", (err) => {
    console.error(`[${name}] gRPC error: ${err.details}`);
    setTimeout(() => listenForEvents(client, name, pda, handlers), 5000);
  });
  stream.on("end", () => {
    console.log(`[${name}] Stream ended. Reconnecting...`);
    setTimeout(() => listenForEvents(client, name, pda, handlers), 5000);
  });
}

module.exports = { createGrpcClient, listenForEvents };