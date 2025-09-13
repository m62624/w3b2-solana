import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { EventEmitter } from 'events';
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

const bridgeProto = grpc.loadPackageDefinition(
  packageDefinition
) as unknown as any;

export class GrpcService extends EventEmitter {
  private server: grpc.Server;
  private client: any;
  private eventStreams: Set<grpc.ServerWritableStream<Empty, BridgeEvent>> =
    new Set();
  private connectorClient: any;
  private isConnected: boolean = false;

  constructor() {
    super();
    this.server = new grpc.Server();
    this.setupServices();
    this.setupConnectorClient();
  }

  private setupServices() {
    // Регистрация BridgeService
    this.server.addService(bridgeProto.bridge.BridgeService.service, {
      streamEvents: this.streamEvents.bind(this),
    });
  }

  private setupConnectorClient() {
    // Создаем клиент для подключения к коннектору
    const connectorUrl = process.env.CONNECTOR_GRPC_URL || 'localhost:50051';
    this.connectorClient = new bridgeProto.bridge.BridgeService(
      connectorUrl,
      grpc.credentials.createInsecure()
    );
  }

  // Подключение к коннектору и получение событий
  public async connectToConnector(): Promise<void> {
    try {
      console.log('🔌 Подключение к gRPC коннектору...');

      const call = this.connectorClient.streamEvents({});

      call.on('data', (event: BridgeEvent) => {
        console.log('📡 Получено событие от коннектора:', event);
        this.isConnected = true;

        // Обогащаем событие дополнительной информацией
        const enrichedEvent = this.enrichEvent(event);

        // Эмитируем событие для других сервисов
        this.emit('blockchain_event', enrichedEvent);

        // Пересылаем событие всем подключенным клиентам
        this.broadcastEvent(enrichedEvent);
      });

      call.on('error', (error: any) => {
        console.error('❌ Ошибка gRPC соединения с коннектором:', error);
        this.isConnected = false;

        // Переподключаемся через 5 секунд
        setTimeout(() => {
          this.connectToConnector();
        }, 5000);
      });

      call.on('end', () => {
        console.log('🔌 Соединение с коннектором закрыто');
        this.isConnected = false;

        // Переподключаемся через 2 секунды
        setTimeout(() => {
          this.connectToConnector();
        }, 2000);
      });

      console.log('✅ Подключение к коннектору установлено');
    } catch (error) {
      console.error('❌ Ошибка подключения к коннектору:', error);
      this.isConnected = false;

      // Переподключаемся через 10 секунд
      setTimeout(() => {
        this.connectToConnector();
      }, 10000);
    }
  }

  // Проверка статуса подключения
  public isConnectorConnected(): boolean {
    return this.isConnected;
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
  public start(port: string = '50052') {
    return new Promise<void>((resolve, reject) => {
      this.server.bindAsync(
        `0.0.0.0:${port}`,
        grpc.ServerCredentials.createInsecure(),
        async (err, port) => {
          if (err) {
            reject(err);
            return;
          }
          this.server.start();
          console.log(`gRPC сервер запущен на порту ${port}`);

          // Подключаемся к коннектору после запуска сервера
          await this.connectToConnector();

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

  // Обогащение события дополнительной информацией
  private enrichEvent(event: BridgeEvent): BridgeEvent & {
    id: string;
    processedAt: number;
    source: string;
    eventType: string;
  } {
    const eventId = this.generateEventId();
    const processedAt = Date.now();
    const source = 'grpc_connector';

    // Определяем тип события
    let eventType = 'unknown';
    if (event.adminRegistered) eventType = 'admin_registered';
    else if (event.userRegistered) eventType = 'user_registered';
    else if (event.adminDeactivated) eventType = 'admin_deactivated';
    else if (event.userDeactivated) eventType = 'user_deactivated';
    else if (event.fundingRequested) eventType = 'funding_requested';
    else if (event.fundingApproved) eventType = 'funding_approved';
    else if (event.commandEvent) eventType = 'command_event';

    return {
      ...event,
      id: eventId,
      processedAt,
      source,
      eventType,
    };
  }

  // Генерация уникального ID события
  private generateEventId(): string {
    return `evt_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  // Получение статистики событий
  public getEventStats() {
    return {
      totalConnections: this.eventStreams.size,
      isConnectorConnected: this.isConnected,
      lastEventTime: Date.now(),
      connectorUrl: process.env.CONNECTOR_GRPC_URL || 'localhost:50051',
    };
  }
}
