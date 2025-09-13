import React, { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { webSocketService, type WebSocketEvent, type ConnectionStatus } from '../services/websocketService';
import toast from 'react-hot-toast';

interface WebSocketContextType {
  isConnected: boolean;
  connectionStatus: ConnectionStatus | null;
  events: WebSocketEvent[];
  requestStatus: () => void;
  clearEvents: () => void;
  reconnect: () => void;
}

const WebSocketContext = createContext<WebSocketContextType | undefined>(undefined);

interface WebSocketProviderProps {
  children: ReactNode;
}

export const WebSocketProvider: React.FC<WebSocketProviderProps> = ({ children }) => {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus | null>(null);
  const [events, setEvents] = useState<WebSocketEvent[]>([]);

  useEffect(() => {
    // Подключаемся к WebSocket серверу
    webSocketService.connect();

    // Обработчики событий
    const handleConnected = () => {
      setIsConnected(true);
      // Автоматически подписываемся на события при подключении
      webSocketService.subscribeToEvents();
      toast.success('Подключение к серверу установлено');
    };

    const handleDisconnected = (data: any) => {
      setIsConnected(false);
      toast.error(`Соединение потеряно: ${data.reason}`);
    };

    const handleBlockchainEvent = (event: any) => {
      const newEvent: WebSocketEvent = {
        type: 'blockchain_event',
        data: event,
        timestamp: Date.now(),
      };
      
      setEvents(prev => [newEvent, ...prev].slice(0, 100)); // Храним последние 100 событий
      
      // Показываем уведомление о событии
      if (event.adminRegistered) {
        toast.success(`Администратор зарегистрирован: ${event.adminRegistered.admin}`);
      } else if (event.admin_registered) {
        toast.success(`Администратор зарегистрирован: ${event.admin_registered.admin}`);
      } else if (event.userRegistered) {
        toast.success(`Пользователь зарегистрирован: ${event.userRegistered.user}`);
      } else if (event.user_registered) {
        toast.success(`Пользователь зарегистрирован: ${event.user_registered.user}`);
      } else if (event.fundingRequested) {
        toast(`Запрос на финансирование: ${event.fundingRequested.amount} lamports`, { icon: '💰' });
      } else if (event.funding_requested) {
        toast(`Запрос на финансирование: ${event.funding_requested.amount} lamports`, { icon: '💰' });
      } else if (event.fundingApproved) {
        toast.success(`Финансирование одобрено: ${event.fundingApproved.amount} lamports`);
      } else if (event.funding_approved) {
        toast.success(`Финансирование одобрено: ${event.funding_approved.amount} lamports`);
      } else if (event.commandEvent) {
        toast(`Команда отправлена: ${event.commandEvent.commandId}`, { icon: '📤' });
      } else if (event.command_event) {
        toast(`Команда отправлена: ${event.command_event.commandId}`, { icon: '📤' });
      }
    };

    const handleNotification = (notification: any) => {
      const newEvent: WebSocketEvent = {
        type: 'notification',
        data: notification,
        timestamp: Date.now(),
      };
      
      setEvents(prev => [newEvent, ...prev].slice(0, 100));
      
      toast(notification.message, {
        icon: notification.type === 'error' ? '❌' : 
              notification.type === 'warning' ? '⚠️' : 
              notification.type === 'success' ? '✅' : 'ℹ️',
      });
    };

    const handleStatus = (status: ConnectionStatus) => {
      setConnectionStatus(status);
    };

    const handleError = (error: any) => {
      console.error('WebSocket ошибка:', error);
      toast.error('Ошибка WebSocket соединения');
    };

    // Подписываемся на события
    webSocketService.on('connected', handleConnected);
    webSocketService.on('disconnected', handleDisconnected);
    webSocketService.on('blockchain_event', handleBlockchainEvent);
    webSocketService.on('notification', handleNotification);
    webSocketService.on('status', handleStatus);
    webSocketService.on('error', handleError);


    // Очистка при размонтировании
    return () => {
      webSocketService.off('connected', handleConnected);
      webSocketService.off('disconnected', handleDisconnected);
      webSocketService.off('blockchain_event', handleBlockchainEvent);
      webSocketService.off('notification', handleNotification);
      webSocketService.off('status', handleStatus);
      webSocketService.off('error', handleError);
    };
  }, []);


  const requestStatus = () => {
    webSocketService.requestStatus();
  };

  const clearEvents = () => {
    setEvents([]);
  };

  const reconnect = () => {
    webSocketService.reconnect();
  };

  const value: WebSocketContextType = {
    isConnected,
    connectionStatus,
    events,
    requestStatus,
    clearEvents,
    reconnect,
  };

  return (
    <WebSocketContext.Provider value={value}>
      {children}
    </WebSocketContext.Provider>
  );
};

export const useWebSocketContext = (): WebSocketContextType => {
  const context = useContext(WebSocketContext);
  if (context === undefined) {
    throw new Error('useWebSocketContext must be used within a WebSocketProvider');
  }
  return context;
};
