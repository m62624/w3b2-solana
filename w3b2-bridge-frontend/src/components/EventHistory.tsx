import React, { useState, useEffect } from 'react';
import { 
  History, 
  Search, 
  Filter, 
  Download, 
  RefreshCw,
  Calendar,
  Database,
  TrendingUp
} from 'lucide-react';
import { apiService } from '../services/apiService';
import toast from 'react-hot-toast';

interface EventHistoryProps {
  className?: string;
}

interface Event {
  id: string;
  eventType: string;
  source: string;
  processedAt: number;
  saved_at: number;
  [key: string]: any;
}

const EventHistory: React.FC<EventHistoryProps> = ({ className = '' }) => {
  const [events, setEvents] = useState<Event[]>([]);
  const [loading, setLoading] = useState(false);
  const [filterType, setFilterType] = useState<string>('all');
  const [searchTerm, setSearchTerm] = useState('');
  const [limit, setLimit] = useState(50);
  const [offset, setOffset] = useState(0);
  const [totalEvents, setTotalEvents] = useState(0);
  const [eventTypes, setEventTypes] = useState<string[]>([]);

  // Загрузка событий
  const loadEvents = async (resetOffset = false) => {
    setLoading(true);
    try {
      const currentOffset = resetOffset ? 0 : offset;
      const response = await apiService.getEvents(limit, currentOffset);
      
      if (response.success && response.data) {
        const newEvents = response.data as Event[];
        setEvents(resetOffset ? newEvents : [...events, ...newEvents]);
        setOffset(currentOffset + newEvents.length);
        setTotalEvents(newEvents.length);
        
        // Обновляем список типов событий
        const types = Array.from(new Set(newEvents.map((e: Event) => e.eventType).filter(Boolean)));
        setEventTypes(prev => Array.from(new Set([...prev, ...types])));
      }
    } catch (error) {
      console.error('Ошибка загрузки событий:', error);
      toast.error('Ошибка загрузки событий');
    } finally {
      setLoading(false);
    }
  };

  // Загрузка событий по типу
  const loadEventsByType = async (type: string) => {
    setLoading(true);
    try {
      const response = await apiService.getEventsByType(type, limit);
      
      if (response.success && response.data) {
        const events = response.data as Event[];
        setEvents(events);
        setOffset(events.length);
        setTotalEvents(events.length);
      }
    } catch (error) {
      console.error('Ошибка загрузки событий по типу:', error);
      toast.error('Ошибка загрузки событий по типу');
    } finally {
      setLoading(false);
    }
  };

  // Загрузка статистики
  const loadStats = async () => {
    try {
      const response = await apiService.getEventStats();
      if (response.success && response.data) {
        setTotalEvents(response.data.events.totalEvents);
      }
    } catch (error) {
      console.error('Ошибка загрузки статистики:', error);
    }
  };

  useEffect(() => {
    loadEvents(true);
    loadStats();
  }, []);

  // Фильтрация событий
  const filteredEvents = events.filter(event => {
    if (filterType !== 'all' && event.eventType !== filterType) {
      return false;
    }
    
    if (searchTerm) {
      const searchLower = searchTerm.toLowerCase();
      const eventText = JSON.stringify(event).toLowerCase();
      return eventText.includes(searchLower);
    }
    
    return true;
  });

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleString('ru-RU');
  };

  const getEventIcon = (event: Event) => {
    // Сначала проверяем eventType
    if (event.eventType && event.eventType !== 'unknown') {
      switch (event.eventType) {
        case 'admin_registered': return '👑';
        case 'user_registered': return '👤';
        case 'funding_requested': return '💰';
        case 'funding_approved': return '✅';
        case 'command_event': return '📤';
        case 'admin_deactivated': return '🔒';
        case 'user_deactivated': return '👤';
      }
    }
    
    // Если eventType unknown или отсутствует, проверяем поля события
    if (event.admin_registered || event.adminRegistered) return '👑';
    if (event.user_registered || event.userRegistered) return '👤';
    if (event.funding_requested || event.fundingRequested) return '💰';
    if (event.funding_approved || event.fundingApproved) return '✅';
    if (event.command_event || event.commandEvent) return '📤';
    if (event.admin_deactivated || event.adminDeactivated) return '🔒';
    if (event.user_deactivated || event.userDeactivated) return '👤';
    
    return '📡';
  };

  const getEventTypeName = (event: Event) => {
    // Сначала проверяем eventType
    if (event.eventType && event.eventType !== 'unknown') {
      return event.eventType.replace('_', ' ').toUpperCase();
    }
    
    // Если eventType unknown или отсутствует, определяем по полям события
    if (event.admin_registered || event.adminRegistered) return 'ADMIN REGISTERED';
    if (event.user_registered || event.userRegistered) return 'USER REGISTERED';
    if (event.funding_requested || event.fundingRequested) return 'FUNDING REQUESTED';
    if (event.funding_approved || event.fundingApproved) return 'FUNDING APPROVED';
    if (event.command_event || event.commandEvent) return 'COMMAND EVENT';
    if (event.admin_deactivated || event.adminDeactivated) return 'ADMIN DEACTIVATED';
    if (event.user_deactivated || event.userDeactivated) return 'USER DEACTIVATED';
    
    return 'НЕИЗВЕСТНОЕ СОБЫТИЕ';
  };

  const handleRefresh = () => {
    loadEvents(true);
    loadStats();
  };

  const handleTypeChange = (type: string) => {
    setFilterType(type);
    if (type === 'all') {
      loadEvents(true);
    } else {
      loadEventsByType(type);
    }
  };

  const handleLoadMore = () => {
    if (filterType === 'all') {
      loadEvents();
    }
  };

  const exportEvents = () => {
    const dataStr = JSON.stringify(filteredEvents, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `events_${new Date().toISOString().split('T')[0]}.json`;
    link.click();
    URL.revokeObjectURL(url);
    toast.success('События экспортированы');
  };

  return (
    <div className={`card ${className}`}>
      <div className="card-header">
        <div className="flex items-center justify-between">
          <h3 className="card-title flex items-center space-x-2">
            <History className="h-5 w-5" />
            <span>История событий</span>
          </h3>
          <div className="flex items-center space-x-2">
            <button
              onClick={handleRefresh}
              disabled={loading}
              className="btn-outline p-2 disabled:opacity-50"
            >
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            </button>
            <button
              onClick={exportEvents}
              className="btn-outline p-2"
            >
              <Download className="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>

      <div className="p-6">
        {/* Фильтры */}
        <div className="mb-6 p-4 bg-slate-800 rounded-lg">
          <h4 className="text-sm font-semibold text-slate-300 mb-3">Фильтры</h4>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label className="block text-xs text-slate-400 mb-2">Тип события</label>
              <select
                value={filterType}
                onChange={(e) => handleTypeChange(e.target.value)}
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
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-slate-400" />
                <input
                  type="text"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  placeholder="Поиск по событиям..."
                  className="w-full pl-10 pr-3 py-2 bg-slate-700 border border-slate-600 rounded text-sm text-slate-300 focus:border-blue-500 focus:outline-none"
                />
              </div>
            </div>
            <div>
              <label className="block text-xs text-slate-400 mb-2">Количество</label>
              <select
                value={limit}
                onChange={(e) => setLimit(parseInt(e.target.value))}
                className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded text-sm text-slate-300 focus:border-blue-500 focus:outline-none"
              >
                <option value={25}>25</option>
                <option value={50}>50</option>
                <option value={100}>100</option>
                <option value={200}>200</option>
              </select>
            </div>
          </div>
        </div>

        {/* Статистика */}
        <div className="mb-6 grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-4 bg-slate-800 rounded-lg">
            <div className="flex items-center space-x-2">
              <Database className="h-5 w-5 text-blue-400" />
              <div>
                <p className="text-sm text-slate-400">Всего событий</p>
                <p className="text-lg font-semibold text-slate-300">{totalEvents}</p>
              </div>
            </div>
          </div>
          <div className="p-4 bg-slate-800 rounded-lg">
            <div className="flex items-center space-x-2">
              <Filter className="h-5 w-5 text-green-400" />
              <div>
                <p className="text-sm text-slate-400">Отфильтровано</p>
                <p className="text-lg font-semibold text-slate-300">{filteredEvents.length}</p>
              </div>
            </div>
          </div>
          <div className="p-4 bg-slate-800 rounded-lg">
            <div className="flex items-center space-x-2">
              <TrendingUp className="h-5 w-5 text-purple-400" />
              <div>
                <p className="text-sm text-slate-400">Типов событий</p>
                <p className="text-lg font-semibold text-slate-300">{eventTypes.length}</p>
              </div>
            </div>
          </div>
        </div>

        {/* Список событий */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-semibold text-slate-300">
              События ({filteredEvents.length})
            </h4>
          </div>

          {filteredEvents.length === 0 ? (
            <div className="text-center py-8">
              <History className="h-12 w-12 text-slate-600 mx-auto mb-4" />
              <p className="text-slate-400">
                {events.length === 0 
                  ? 'События не найдены' 
                  : 'События не найдены по заданным фильтрам'
                }
              </p>
            </div>
          ) : (
            <div className="max-h-96 overflow-y-auto space-y-2">
              {filteredEvents.map((event, index) => (
                <div
                  key={event.id || index}
                  className="p-3 bg-slate-800 rounded-lg border border-slate-700 hover:border-slate-600 transition-colors"
                >
                  <div className="flex items-start space-x-3">
                    <div className="text-lg">{getEventIcon(event)}</div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between">
                        <p className="text-sm text-slate-300">
                          {getEventTypeName(event)}
                        </p>
                        <span className="text-xs text-slate-500">
                          {formatTimestamp(event.processedAt || event.saved_at)}
                        </span>
                      </div>
                      <div className="mt-1 text-xs text-slate-500">
                        ID: {event.id?.slice(0, 12)}... | Источник: {event.source}
                      </div>
                      <div className="mt-2 text-xs text-slate-400">
                        <pre className="whitespace-pre-wrap break-words">
                          {JSON.stringify(event, null, 2).slice(0, 200)}...
                        </pre>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Кнопка загрузки еще */}
          {filterType === 'all' && filteredEvents.length > 0 && (
            <div className="text-center pt-4">
              <button
                onClick={handleLoadMore}
                disabled={loading}
                className="btn-outline disabled:opacity-50"
              >
                {loading ? 'Загрузка...' : 'Загрузить еще'}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default EventHistory;
