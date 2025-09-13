import React, { useState, useEffect } from 'react';
import { 
  Settings as SettingsIcon, 
  Save, 
  RefreshCw, 
  Globe, 
  Key, 
  Database,
  Shield,
  Info,
  CheckCircle,
  AlertCircle
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';
import { useApiContext } from '../contexts/ApiContext';
import toast from 'react-hot-toast';

const Settings: React.FC = () => {
  const { checkConnection } = useWalletContext();
  const { healthCheck, getStats } = useApiContext();
  
  const [settings, setSettings] = useState({
    solanaRpcUrl: process.env.REACT_APP_SOLANA_RPC_URL || 'https://api.devnet.solana.com',
    programId: process.env.REACT_APP_PROGRAM_ID || 'W3B2Bridge111111111111111111111111111111111',
    apiUrl: process.env.REACT_APP_API_URL || 'http://localhost:3001/api',
    autoRefresh: true,
    refreshInterval: 30,
  });

  const [systemStatus, setSystemStatus] = useState({
    solanaConnected: false,
    apiConnected: false,
    lastCheck: null as Date | null,
  });

  const [isLoading, setIsLoading] = useState(false);
  const [stats, setStats] = useState<any>(null);

  useEffect(() => {
    checkSystemStatus();
    loadStats();
  }, []);

  const checkSystemStatus = async () => {
    try {
      setIsLoading(true);
      
      const [solanaConnected, apiConnected] = await Promise.all([
        checkConnection(),
        healthCheck(),
      ]);

      setSystemStatus({
        solanaConnected,
        apiConnected,
        lastCheck: new Date(),
      });
    } catch (error) {
      console.error('Ошибка проверки статуса:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const loadStats = async () => {
    try {
      const response = await getStats();
      if (response.success) {
        setStats(response.data);
      }
    } catch (error) {
      console.error('Ошибка загрузки статистики:', error);
    }
  };

  const handleSaveSettings = () => {
    // В реальном приложении здесь бы сохранялись настройки в localStorage или на сервере
    localStorage.setItem('w3b2-settings', JSON.stringify(settings));
    toast.success('Настройки сохранены');
  };

  const handleResetSettings = () => {
    const defaultSettings = {
      solanaRpcUrl: 'https://api.devnet.solana.com',
      programId: 'W3B2Bridge111111111111111111111111111111111',
      apiUrl: 'http://localhost:3001/api',
      autoRefresh: true,
      refreshInterval: 30,
    };
    setSettings(defaultSettings);
    toast.success('Настройки сброшены');
  };

  const handleExportSettings = () => {
    const dataStr = JSON.stringify(settings, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'w3b2-settings.json';
    link.click();
    URL.revokeObjectURL(url);
    toast.success('Настройки экспортированы');
  };

  const handleImportSettings = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const importedSettings = JSON.parse(e.target?.result as string);
        setSettings(importedSettings);
        toast.success('Настройки импортированы');
      } catch (error) {
        toast.error('Ошибка импорта настроек');
      }
    };
    reader.readAsText(file);
  };

  return (
    <div className="space-y-6">
      {/* Заголовок */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Настройки</h1>
          <p className="text-slate-400 mt-1">
            Конфигурация W3B2 Bridge Protocol
          </p>
        </div>
        <button
          onClick={checkSystemStatus}
          disabled={isLoading}
          className="btn-outline flex items-center space-x-2"
        >
          <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
          <span>Проверить статус</span>
        </button>
      </div>

      {/* Статус системы */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="card">
          <div className="flex items-center space-x-3">
            <div className={`p-2 rounded-lg ${systemStatus.solanaConnected ? 'bg-green-500/10' : 'bg-red-500/10'}`}>
              <Globe className={`h-6 w-6 ${systemStatus.solanaConnected ? 'text-green-500' : 'text-red-500'}`} />
            </div>
            <div>
              <h3 className="text-lg font-semibold text-white">Solana Network</h3>
              <p className={`text-sm ${systemStatus.solanaConnected ? 'text-green-400' : 'text-red-400'}`}>
                {systemStatus.solanaConnected ? 'Подключено' : 'Отключено'}
              </p>
              {systemStatus.lastCheck && (
                <p className="text-xs text-slate-400">
                  Последняя проверка: {systemStatus.lastCheck.toLocaleTimeString()}
                </p>
              )}
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center space-x-3">
            <div className={`p-2 rounded-lg ${systemStatus.apiConnected ? 'bg-green-500/10' : 'bg-red-500/10'}`}>
              <Database className={`h-6 w-6 ${systemStatus.apiConnected ? 'text-green-500' : 'text-red-500'}`} />
            </div>
            <div>
              <h3 className="text-lg font-semibold text-white">API Server</h3>
              <p className={`text-sm ${systemStatus.apiConnected ? 'text-green-400' : 'text-red-400'}`}>
                {systemStatus.apiConnected ? 'Подключено' : 'Отключено'}
              </p>
              {systemStatus.lastCheck && (
                <p className="text-xs text-slate-400">
                  Последняя проверка: {systemStatus.lastCheck.toLocaleTimeString()}
                </p>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Настройки сети */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Настройки сети</h3>
          <p className="card-subtitle">Конфигурация подключений к блокчейну и API</p>
        </div>
        
        <div className="space-y-4">
          <div className="form-group">
            <label className="form-label">Solana RPC URL</label>
            <input
              type="url"
              value={settings.solanaRpcUrl}
              onChange={(e) => setSettings({ ...settings, solanaRpcUrl: e.target.value })}
              className="form-input"
              placeholder="https://api.devnet.solana.com"
            />
          </div>
          
          <div className="form-group">
            <label className="form-label">Program ID</label>
            <input
              type="text"
              value={settings.programId}
              onChange={(e) => setSettings({ ...settings, programId: e.target.value })}
              className="form-input font-mono"
              placeholder="W3B2Bridge111111111111111111111111111111111"
            />
          </div>
          
          <div className="form-group">
            <label className="form-label">API URL</label>
            <input
              type="url"
              value={settings.apiUrl}
              onChange={(e) => setSettings({ ...settings, apiUrl: e.target.value })}
              className="form-input"
              placeholder="http://localhost:3001/api"
            />
          </div>
        </div>
      </div>

      {/* Настройки приложения */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Настройки приложения</h3>
          <p className="card-subtitle">Поведение и интерфейс</p>
        </div>
        
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium text-white">Автообновление</label>
              <p className="text-xs text-slate-400">Автоматически обновлять данные</p>
            </div>
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={settings.autoRefresh}
                onChange={(e) => setSettings({ ...settings, autoRefresh: e.target.checked })}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-slate-600 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
            </label>
          </div>
          
          {settings.autoRefresh && (
            <div className="form-group">
              <label className="form-label">Интервал обновления (секунды)</label>
              <input
                type="number"
                min="5"
                max="300"
                value={settings.refreshInterval}
                onChange={(e) => setSettings({ ...settings, refreshInterval: parseInt(e.target.value) })}
                className="form-input"
              />
            </div>
          )}
        </div>
      </div>

      {/* Статистика системы */}
      {stats && (
        <div className="card">
          <div className="card-header">
            <h3 className="card-title">Статистика системы</h3>
            <p className="card-subtitle">Текущее состояние W3B2 Bridge</p>
          </div>
          
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="text-center">
              <div className="text-2xl font-bold text-blue-500">{stats.database?.users || 0}</div>
              <div className="text-sm text-slate-400">Пользователи</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-green-500">{stats.database?.fundingRequests || 0}</div>
              <div className="text-sm text-slate-400">Запросы</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-purple-500">{stats.database?.records || 0}</div>
              <div className="text-sm text-slate-400">Записи</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-orange-500">{stats.sessions?.active || 0}</div>
              <div className="text-sm text-slate-400">Сессии</div>
            </div>
          </div>
        </div>
      )}

      {/* Информация о версии */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Информация о версии</h3>
          <p className="card-subtitle">Детали приложения и протокола</p>
        </div>
        
        <div className="space-y-3">
          <div className="flex justify-between">
            <span className="text-slate-400">Версия приложения</span>
            <span className="text-white font-mono">1.0.0</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Версия протокола</span>
            <span className="text-white font-mono">W3B2 v1.0</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Сеть Solana</span>
            <span className="text-white font-mono">Devnet</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Последнее обновление</span>
            <span className="text-white font-mono">{new Date().toLocaleDateString()}</span>
          </div>
        </div>
      </div>

      {/* Действия с настройками */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Управление настройками</h3>
          <p className="card-subtitle">Сохранение, сброс и экспорт конфигурации</p>
        </div>
        
        <div className="flex flex-wrap gap-3">
          <button
            onClick={handleSaveSettings}
            className="btn-primary flex items-center space-x-2"
          >
            <Save className="h-4 w-4" />
            <span>Сохранить</span>
          </button>
          
          <button
            onClick={handleResetSettings}
            className="btn-outline flex items-center space-x-2"
          >
            <RefreshCw className="h-4 w-4" />
            <span>Сбросить</span>
          </button>
          
          <button
            onClick={handleExportSettings}
            className="btn-secondary flex items-center space-x-2"
          >
            <Database className="h-4 w-4" />
            <span>Экспорт</span>
          </button>
          
          <label className="btn-outline flex items-center space-x-2 cursor-pointer">
            <Key className="h-4 w-4" />
            <span>Импорт</span>
            <input
              type="file"
              accept=".json"
              onChange={handleImportSettings}
              className="hidden"
            />
          </label>
        </div>
      </div>
    </div>
  );
};

export default Settings;
