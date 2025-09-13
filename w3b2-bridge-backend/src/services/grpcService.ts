import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { BridgeEvent, Empty, CommandMode } from '../types/bridge.proto';

// Получение __dirname в ES модулях
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Загрузка proto файла
const PROTO_PATH = join(__dirname, '../../proto/bridge.proto');
const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
});

const bridgeProto = grpc.loadPackageDefinition(packageDefinition) as any;

export class GrpcService {
  private server: grpc.Server;
  private eventStreams: Set<grpc.ServerWritableStream<Empty, BridgeEvent>> =
    new Set();

  constructor() {
    this.server = new grpc.Server();
    this.setupServices();
  }

  private setupServices() {
    // Регистрация BridgeService
    this.server.addService(bridgeProto.bridge.BridgeService.service, {
      streamEvents: this.streamEvents.bind(this),
    });
  }

  // Обработчик для стриминга событий
  private streamEvents(call: grpc.ServerWritableStream<Empty, BridgeEvent>) {
    console.log('Новый клиент подключился к стриму событий');

    // Добавляем клиента в список активных стримов
    this.eventStreams.add(call);

    // Обработка отключения клиента
    call.on('cancelled', () => {
      console.log('Клиент отключился от стрима событий');
      this.eventStreams.delete(call);
    });

    // Отправляем приветственное событие
    const welcomeEvent: BridgeEvent = {
      adminRegistered: {
        admin: 'system',
        initialFunding: 0,
        ts: Date.now(),
      },
    };

    call.write(welcomeEvent);
  }

  // Метод для отправки событий всем подключенным клиентам
  public broadcastEvent(event: BridgeEvent) {
    console.log('Отправка события всем подключенным клиентам:', event);

    this.eventStreams.forEach(stream => {
      try {
        stream.write(event);
      } catch (error) {
        console.error('Ошибка при отправке события клиенту:', error);
        this.eventStreams.delete(stream);
      }
    });
  }

  // Методы для создания различных типов событий
  public createAdminRegisteredEvent(
    admin: string,
    initialFunding: number
  ): BridgeEvent {
    return {
      adminRegistered: {
        admin,
        initialFunding,
        ts: Date.now(),
      },
    };
  }

  public createUserRegisteredEvent(
    user: string,
    initialBalance: number
  ): BridgeEvent {
    return {
      userRegistered: {
        user,
        initialBalance,
        ts: Date.now(),
      },
    };
  }

  public createAdminDeactivatedEvent(admin: string): BridgeEvent {
    return {
      adminDeactivated: {
        admin,
        ts: Date.now(),
      },
    };
  }

  public createUserDeactivatedEvent(user: string): BridgeEvent {
    return {
      userDeactivated: {
        user,
        ts: Date.now(),
      },
    };
  }

  public createFundingRequestedEvent(
    userWallet: string,
    targetAdmin: string,
    amount: number
  ): BridgeEvent {
    return {
      fundingRequested: {
        userWallet,
        targetAdmin,
        amount,
        ts: Date.now(),
      },
    };
  }

  public createFundingApprovedEvent(
    userWallet: string,
    approvedBy: string,
    amount: number
  ): BridgeEvent {
    return {
      fundingApproved: {
        userWallet,
        approvedBy,
        amount,
        ts: Date.now(),
      },
    };
  }

  public createCommandEvent(
    sender: string,
    target: string,
    commandId: number,
    mode: CommandMode,
    payload: Uint8Array
  ): BridgeEvent {
    return {
      commandEvent: {
        sender,
        target,
        commandId,
        mode,
        payload,
        ts: Date.now(),
      },
    };
  }

  // Запуск gRPC сервера
  public start(port: string = '50051') {
    return new Promise<void>((resolve, reject) => {
      this.server.bindAsync(
        `0.0.0.0:${port}`,
        grpc.ServerCredentials.createInsecure(),
        (err, port) => {
          if (err) {
            reject(err);
            return;
          }
          this.server.start();
          console.log(`gRPC сервер запущен на порту ${port}`);
          resolve();
        }
      );
    });
  }

  // Остановка gRPC сервера
  public stop() {
    return new Promise<void>(resolve => {
      this.server.forceShutdown();
      console.log('gRPC сервер остановлен');
      resolve();
    });
  }

  // Получение количества активных подключений
  public getActiveConnectionsCount(): number {
    return this.eventStreams.size;
  }
}
