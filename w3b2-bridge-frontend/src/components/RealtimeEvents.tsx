import React, { useState } from 'react';
import { 
  Activity, 
  Wifi, 
  WifiOff, 
  RefreshCw, 
  Trash2, 
  Eye, 
  EyeOff,
  Clock,
  Users,
  Database
} from 'lucide-react';
import { useWebSocketContext } from '../contexts/WebSocketContext';
import toast from 'react-hot-toast';

const RealtimeEvents: React.FC = () => {
  const { 
    isConnected, 
    connectionStatus, 
    events, 
    requestStatus, 
    clearEvents, 
    reconnect 
  } = useWebSocketContext();

  const [showEvents, setShowEvents] = useState(true);
  const [autoScroll, setAutoScroll] = useState(true);
  const [filterType, setFilterType] = useState<string>('all');
  const [searchTerm, setSearchTerm] = useState('');

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString('ru-RU');
  };

  const getEventIcon = (event: any) => {
    if (event.adminRegistered) return '👑';
    if (event.userRegistered) return '👤';
    if (event.fundingRequested) return '💰';
    if (event.fundingApproved) return '✅';
    if (event.commandEvent) return '📤';
    if (event.adminDeactivated) return '🔒';
    if (event.userDeactivated) return '👤';
    return '📡';
  };

  const getEventDescription = (event: any) => {
    if (event.adminRegistered) {
      return `Администратор зарегистрирован: ${event.adminRegistered.admin.slice(0, 8)}...`;
    }
    if (event.userRegistered) {
      return `Пользователь зарегистрирован: ${event.userRegistered.user.slice(0, 8)}...`;
    }
    if (event.fundingRequested) {
      return `Запрос на финансирование: ${(event.fundingRequested.amount / 1000000000).toFixed(4)} SOL`;
    }
    if (event.fundingApproved) {
      return `Финансирование одобрено: ${(event.fundingApproved.amount / 1000000000).toFixed(4)} SOL`;
    }
    if (event.commandEvent) {
      return `Команда отправлена: ID ${event.commandEvent.commandId}`;
    }
    if (event.adminDeactivated) {
      return `Администратор деактивирован: ${event.adminDeactivated.admin.slice(0, 8)}...`;
    }
    if (event.userDeactivated) {
      return `Пользователь деактивирован: ${event.userDeactivated.user.slice(0, 8)}...`;
    }
    return 'Неизвестное событие';
  };

  const getEventDetails = (event: any) => {
    const details = [];
    
    if (event.id) {
      details.push(`ID: ${event.id.slice(0, 12)}...`);
    }
    
    if (event.eventType) {
      details.push(`Тип: ${event.eventType}`);
    }
    
    if (event.source) {
      details.push(`Источник: ${event.source}`);
    }
    
    if (event.processedAt) {
      details.push(`Обработано: ${formatTimestamp(event.processedAt)}`);
    }
    
    return details;
  };


  const handleClearEvents = () => {
    clearEvents();
    toast.success('События очищены');
  };

  const handleReconnect = () => {
    reconnect();
    toast('Переподключение...', { icon: '🔄' });
  };

  const handleRequestStatus = () => {
    requestStatus();
    toast('Запрос статуса отправлен', { icon: 'ℹ️' });
  };

  // Фильтрация событий
  const filteredEvents = events.filter(event => {
    // Фильтр по типу
    if (filterType !== 'all' && event.data.eventType !== filterType) {
      return false;
    }
    
    // Поиск по тексту
    if (searchTerm) {
      const searchLower = searchTerm.toLowerCase();
      const eventText = getEventDescription(event.data).toLowerCase();
      const eventType = event.data.eventType?.toLowerCase() || '';
      const eventId = event.data.id?.toLowerCase() || '';
      
      return eventText.includes(searchLower) || 
             eventType.includes(searchLower) || 
             eventId.includes(searchLower);
    }
    
    return true;
  });

  // Получение уникальных типов событий для фильтра
  const eventTypes = Array.from(new Set(events.map(event => event.data.eventType).filter(Boolean)));

  return (
    <div className="card">
      <div className="card-header">
        <div className="flex items-center justify-between">
          <h3 className="card-title flex items-center space-x-2">
            <Activity className="h-5 w-5" />
            <span>Real-time события</span>
          </h3>
          <div className="flex items-center space-x-2">
            <div className={`flex items-center space-x-1 px-2 py-1 rounded-full text-xs ${
              isConnected 
                ? 'bg-green-500/20 text-green-300' 
                : 'bg-red-500/20 text-red-300'
            }`}>
              {isConnected ? <Wifi className="h-3 w-3" /> : <WifiOff className="h-3 w-3" />}
              <span>{isConnected ? 'Подключено' : 'Отключено'}</span>
            </div>
            <button
              onClick={() => setShowEvents(!showEvents)}
              className="btn-outline p-2"
            >
              {showEvents ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </div>

      <div className="p-6">
        {/* Статус подключения */}
        <div className="mb-6 p-4 bg-slate-800 rounded-lg">
          <div className="flex items-center justify-between mb-4">
            <h4 className="text-sm font-semibold text-slate-300">Статус соединения</h4>
            <div className="flex space-x-2">
              <button
                onClick={handleRequestStatus}
                className="btn-outline text-xs px-3 py-1"
              >
                <RefreshCw className="h-3 w-3 mr-1" />
                Обновить
              </button>
              <button
                onClick={handleReconnect}
                className="btn-outline text-xs px-3 py-1"
              >
                <Wifi className="h-3 w-3 mr-1" />
                Переподключить
              </button>
            </div>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div className="flex items-center space-x-2">
              <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`}></div>
              <span className="text-slate-400">WebSocket:</span>
              <span className={isConnected ? 'text-green-300' : 'text-red-300'}>
                {isConnected ? 'Активно' : 'Неактивно'}
              </span>
            </div>
            
            {connectionStatus && (
              <>
                <div className="flex items-center space-x-2">
                  <Users className="h-4 w-4 text-slate-400" />
                  <span className="text-slate-400">Клиенты:</span>
                  <span className="text-slate-300">{connectionStatus.clientsCount}</span>
                </div>
                
                <div className="flex items-center space-x-2">
                  <Clock className="h-4 w-4 text-slate-400" />
                  <span className="text-slate-400">Обновлено:</span>
                  <span className="text-slate-300">
                    {formatTimestamp(connectionStatus.timestamp)}
                  </span>
                </div>
              </>
            )}
          </div>
        </div>

        {/* Управление событиями */}
        <div className="mb-6 p-4 bg-slate-800 rounded-lg">
          <h4 className="text-sm font-semibold text-slate-300 mb-3">Управление событиями</h4>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={handleClearEvents}
              className="btn-outline text-sm px-4 py-2 text-red-300 hover:bg-red-500/20"
            >
              <Trash2 className="h-4 w-4 mr-1" />
              Очистить события
            </button>
          </div>
        </div>

        {/* Фильтры и поиск */}
        {showEvents && (
          <div className="mb-6 p-4 bg-slate-800 rounded-lg">
            <h4 className="text-sm font-semibold text-slate-300 mb-3">Фильтры и поиск</h4>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-slate-400 mb-2">Тип события</label>
                <select
                  value={filterType}
                  onChange={(e) => setFilterType(e.target.value)}
                  className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded text-sm text-slate-300 focus:border-blue-500 focus:outline-none"
                >
                  <option value="all">Все типы</option>
                  {eventTypes.map(type => (
                    <option key={type} value={type}>
                      {type.replace('_', ' ').toUpperCase()}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-xs text-slate-400 mb-2">Поиск</label>
                <input
                  type="text"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  placeholder="Поиск по событиям..."
                  className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded text-sm text-slate-300 focus:border-blue-500 focus:outline-none"
                />
              </div>
            </div>
          </div>
        )}

        {/* Список событий */}
        {showEvents && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-sm font-semibold text-slate-300">
                События ({filteredEvents.length} из {events.length})
              </h4>
              <label className="flex items-center space-x-2 text-sm text-slate-400">
                <input
                  type="checkbox"
                  checked={autoScroll}
                  onChange={(e) => setAutoScroll(e.target.checked)}
                  className="rounded"
                />
                <span>Автопрокрутка</span>
              </label>
            </div>

            {filteredEvents.length === 0 ? (
              <div className="text-center py-8">
                <Database className="h-12 w-12 text-slate-600 mx-auto mb-4" />
                <p className="text-slate-400">
                  {events.length === 0 
                    ? 'События не получены' 
                    : 'События не найдены по заданным фильтрам'
                  }
                </p>
                <p className="text-sm text-slate-500 mt-1">
                  {events.length === 0 
                    ? 'Подключитесь к серверу для получения событий'
                    : 'Попробуйте изменить фильтры или поисковый запрос'
                  }
                </p>
              </div>
            ) : (
              <div className="max-h-96 overflow-y-auto space-y-2">
                {filteredEvents.map((event, index) => (
                  <div
                    key={index}
                    className="p-3 bg-slate-800 rounded-lg border border-slate-700 hover:border-slate-600 transition-colors"
                  >
                    <div className="flex items-start space-x-3">
                      <div className="text-lg">{getEventIcon(event.data)}</div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center justify-between">
                          <p className="text-sm text-slate-300 truncate">
                            {getEventDescription(event.data)}
                          </p>
                          <span className="text-xs text-slate-500 ml-2">
                            {formatTimestamp(event.timestamp)}
                          </span>
                        </div>
                        <div className="mt-1 text-xs text-slate-500">
                          Тип: {event.type}
                        </div>
                        {getEventDetails(event.data).length > 0 && (
                          <div className="mt-2 space-y-1">
                            {getEventDetails(event.data).map((detail, idx) => (
                              <div key={idx} className="text-xs text-slate-400">
                                {detail}
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default RealtimeEvents;
