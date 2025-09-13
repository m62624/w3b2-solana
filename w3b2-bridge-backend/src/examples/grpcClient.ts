import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { BridgeEvent } from '../types/bridge.proto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PROTO_PATH = join(__dirname, '../../proto/bridge.proto');
const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
});

const bridgeProto = grpc.loadPackageDefinition(packageDefinition) as any;

export class GrpcClient {
  private client: any;

  constructor(serverUrl: string = 'localhost:50051') {
    this.client = new bridgeProto.bridge.BridgeService(
      serverUrl,
      grpc.credentials.createInsecure()
    );
  }

  public async connectToEventStream(): Promise<void> {
    console.log('🔌 Подключение к gRPC серверу...');

    const call = this.client.streamEvents({});

    call.on('data', (event: BridgeEvent) => {
      console.log('📨 Получено событие:', JSON.stringify(event, null, 2));
    });

    call.on('end', () => {
      console.log('🔌 Соединение с сервером закрыто');
    });

    call.on('error', (error: Error) => {
      console.error('❌ Ошибка gRPC соединения:', error);
    });

    process.on('SIGINT', () => {
      console.log('\n🛑 Отключение от сервера...');
      call.cancel();
      process.exit(0);
    });
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const client = new GrpcClient();
  client.connectToEventStream().catch(console.error);
}
