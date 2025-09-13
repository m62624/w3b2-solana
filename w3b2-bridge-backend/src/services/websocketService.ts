import { Server as SocketIOServer } from 'socket.io';
import { Server as HTTPServer } from 'http';
import { EventEmitter } from 'events';
import { BridgeEvent } from '../types/bridge.proto';

export class WebSocketService extends EventEmitter {
  private io: SocketIOServer;
  private connectedClients: Map<string, any> = new Map();

  constructor(server: HTTPServer) {
    super();

    this.io = new SocketIOServer(server, {
      cors: {
        origin: process.env.FRONTEND_URL || 'http://localhost:3000',
        methods: ['GET', 'POST'],
        credentials: true,
      },
      transports: ['websocket', 'polling'],
    });

    this.setupEventHandlers();
  }

  private setupEventHandlers() {
    this.io.on('connection', socket => {
      console.log(`🔌 Клиент подключился: ${socket.id}`);
      this.connectedClients.set(socket.id, socket);

      // Отправляем приветственное сообщение
      socket.emit('connected', {
        message: 'Подключение к W3B2 Bridge установлено',
        timestamp: Date.now(),
      });

      // Обработка отключения
      socket.on('disconnect', reason => {
        console.log(`🔌 Клиент отключился: ${socket.id}, причина: ${reason}`);
        this.connectedClients.delete(socket.id);
      });

      // Обработка подписки на события
      socket.on('subscribe_events', data => {
        console.log(`📡 Клиент ${socket.id} подписался на события:`, data);
        socket.join('blockchain_events');
      });

      // Обработка отписки от событий
      socket.on('unsubscribe_events', () => {
        console.log(`📡 Клиент ${socket.id} отписался от событий`);
        socket.leave('blockchain_events');
      });

      // Обработка запроса статуса
      socket.on('get_status', () => {
        socket.emit('status', {
          connected: true,
          clientsCount: this.connectedClients.size,
          timestamp: Date.now(),
        });
      });
    });
  }

  // Отправка события всем подключенным клиентам
  public broadcastEvent(event: BridgeEvent) {
    console.log('📡 Отправка события всем клиентам:', event);
    this.io.to('blockchain_events').emit('blockchain_event', event);
  }

  // Отправка события конкретному клиенту
  public sendToClient(clientId: string, event: string, data: any) {
    const client = this.connectedClients.get(clientId);
    if (client) {
      client.emit(event, data);
    }
  }

  // Отправка уведомления всем клиентам
  public broadcastNotification(
    type: 'success' | 'error' | 'warning' | 'info',
    message: string
  ) {
    this.io.emit('notification', {
      type,
      message,
      timestamp: Date.now(),
    });
  }

  // Получение статистики подключений
  public getConnectionStats() {
    return {
      totalClients: this.connectedClients.size,
      subscribedClients:
        this.io.sockets.adapter.rooms.get('blockchain_events')?.size || 0,
      timestamp: Date.now(),
    };
  }

  // Остановка сервиса
  public stop() {
    this.io.close();
    console.log('🔌 WebSocket сервис остановлен');
  }
}
