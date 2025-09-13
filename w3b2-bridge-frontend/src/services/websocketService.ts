import { io, Socket } from 'socket.io-client';
import { EventEmitter } from 'events';

export interface WebSocketEvent {
  type: 'blockchain_event' | 'notification' | 'status' | 'connected';
  data: any;
  timestamp: number;
}

export interface ConnectionStatus {
  connected: boolean;
  clientsCount: number;
  timestamp: number;
}

export class WebSocketService extends EventEmitter {
  private socket: Socket | null = null;
  private isConnected: boolean = false;
  private reconnectAttempts: number = 0;
  private maxReconnectAttempts: number = 5;
  private reconnectInterval: number = 5000;

  constructor() {
    super();
  }

  // Подключение к WebSocket серверу
  public connect(serverUrl: string = process.env.REACT_APP_WS_URL || 'http://localhost:3001') {
    if (this.socket?.connected) {
      console.log('WebSocket уже подключен');
      return;
    }

    console.log('🔌 Подключение к WebSocket серверу:', serverUrl);

    this.socket = io(serverUrl, {
      transports: ['websocket', 'polling'],
      timeout: 10000,
      forceNew: true,
    });

    this.setupEventHandlers();
  }

  private setupEventHandlers() {
    if (!this.socket) return;

    // Подключение установлено
    this.socket.on('connect', () => {
      console.log('✅ WebSocket подключен:', this.socket?.id);
      this.isConnected = true;
      this.reconnectAttempts = 0;
      this.emit('connected', { connected: true, timestamp: Date.now() });
    });

    // Отключение
    this.socket.on('disconnect', (reason: string) => {
      console.log('❌ WebSocket отключен:', reason);
      this.isConnected = false;
      this.emit('disconnected', { reason, timestamp: Date.now() });
      
      // Автоматическое переподключение
      if (this.reconnectAttempts < this.maxReconnectAttempts) {
        this.reconnectAttempts++;
        console.log(`🔄 Попытка переподключения ${this.reconnectAttempts}/${this.maxReconnectAttempts}`);
        setTimeout(() => {
          this.connect();
        }, this.reconnectInterval);
      }
    });

    // Ошибка подключения
    this.socket.on('connect_error', (error: Error) => {
      console.error('❌ Ошибка подключения WebSocket:', error);
      this.emit('error', { error, timestamp: Date.now() });
    });

    // События от сервера
    this.socket.on('blockchain_event', (event: any) => {
      console.log('📡 Получено событие блокчейна:', event);
      this.emit('blockchain_event', event);
    });

    this.socket.on('notification', (notification: any) => {
      console.log('🔔 Получено уведомление:', notification);
      this.emit('notification', notification);
    });

    this.socket.on('status', (status: any) => {
      console.log('📊 Статус сервера:', status);
      this.emit('status', status);
    });

    this.socket.on('connected', (data: any) => {
      console.log('🎉 Подключение подтверждено:', data);
      this.emit('server_connected', data);
    });
  }

  // Подписка на события блокчейна
  public subscribeToEvents() {
    if (this.socket?.connected) {
      this.socket.emit('subscribe_events', { timestamp: Date.now() });
      console.log('📡 Подписка на события блокчейна');
    }
  }

  // Отписка от событий блокчейна
  public unsubscribeFromEvents() {
    if (this.socket?.connected) {
      this.socket.emit('unsubscribe_events');
      console.log('📡 Отписка от событий блокчейна');
    }
  }

  // Запрос статуса сервера
  public requestStatus() {
    if (this.socket?.connected) {
      this.socket.emit('get_status');
    }
  }

  // Отправка произвольного сообщения
  public sendMessage(event: string, data: any) {
    if (this.socket?.connected) {
      this.socket.emit(event, data);
    }
  }

  // Получение статуса подключения
  public getConnectionStatus(): boolean {
    return this.isConnected && this.socket?.connected === true;
  }

  // Получение ID сокета
  public getSocketId(): string | undefined {
    return this.socket?.id;
  }

  // Отключение
  public disconnect() {
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
      this.isConnected = false;
      console.log('🔌 WebSocket отключен');
    }
  }

  // Принудительное переподключение
  public reconnect() {
    this.disconnect();
    setTimeout(() => {
      this.connect();
    }, 1000);
  }

  // Очистка всех обработчиков
  public cleanup() {
    this.removeAllListeners();
    this.disconnect();
  }
}

// Создаем singleton instance
export const webSocketService = new WebSocketService();
export default webSocketService;
