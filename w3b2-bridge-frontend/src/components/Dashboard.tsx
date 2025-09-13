import React, { useState, useEffect } from 'react';
import { 
  Wallet, 
  CreditCard, 
  Database, 
  Activity, 
  Shield,
  Users,
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';
import { useApiContext } from '../contexts/ApiContext';
import type { AppStats } from '../types/index';
import toast from 'react-hot-toast';

const Dashboard: React.FC = () => {
  const { walletInfo, balance, refreshBalance, getRecentTransactions } = useWalletContext();
  const { getStats, healthCheck } = useApiContext();
  const [stats, setStats] = useState<AppStats | null>(null);
  const [recentTransactions, setRecentTransactions] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [apiHealth, setApiHealth] = useState<boolean>(false);

  useEffect(() => {
    loadDashboardData();
  }, []);

  const loadDashboardData = async () => {
    try {
      setIsLoading(true);
      
      // Загружаем статистику
      const statsResponse = await getStats();
      if (statsResponse.success) {
        setStats(statsResponse.data);
      }

      // Проверяем состояние API
      const health = await healthCheck();
      setApiHealth(health);

      // Загружаем последние транзакции
      if (walletInfo.connected) {
        const transactions = await getRecentTransactions(5);
        setRecentTransactions(transactions);
      }
    } catch (error) {
      console.error('Ошибка загрузки данных:', error);
      toast.error('Ошибка загрузки данных');
    } finally {
      setIsLoading(false);
    }
  };

  const handleRefresh = async () => {
    await refreshBalance();
    await loadDashboardData();
    toast.success('Данные обновлены');
  };

  const statCards = [
    {
      title: 'Баланс кошелька',
      value: `${balance.toFixed(4)} SOL`,
      icon: Wallet,
      color: 'text-blue-500',
      bgColor: 'bg-blue-500/10',
    },
    {
      title: 'Запросы на финансирование',
      value: stats?.database.fundingRequests || 0,
      icon: CreditCard,
      color: 'text-green-500',
      bgColor: 'bg-green-500/10',
    },
    {
      title: 'Записи в базе данных',
      value: stats?.database.records || 0,
      icon: Database,
      color: 'text-purple-500',
      bgColor: 'bg-purple-500/10',
    },
    {
      title: 'Активные сессии',
      value: stats?.sessions.active || 0,
      icon: Activity,
      color: 'text-orange-500',
      bgColor: 'bg-orange-500/10',
    },
  ];

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-96">
        <div className="text-center">
          <div className="loading-spinner mx-auto mb-4"></div>
          <p className="text-slate-400">Загрузка данных...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Заголовок */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Панель управления</h1>
          <p className="text-slate-400 mt-1">
            Добро пожаловать в W3B2 Bridge Protocol
          </p>
        </div>
        <button
          onClick={handleRefresh}
          className="btn-primary flex items-center space-x-2"
        >
          <Activity className="h-4 w-4" />
          <span>Обновить</span>
        </button>
      </div>

      {/* Статус системы */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="card">
          <div className="flex items-center space-x-3">
            <div className={`p-2 rounded-lg ${apiHealth ? 'bg-green-500/10' : 'bg-red-500/10'}`}>
              <Shield className={`h-6 w-6 ${apiHealth ? 'text-green-500' : 'text-red-500'}`} />
            </div>
            <div>
              <h3 className="text-lg font-semibold text-white">Статус API</h3>
              <p className={`text-sm ${apiHealth ? 'text-green-400' : 'text-red-400'}`}>
                {apiHealth ? 'Подключено' : 'Отключено'}
              </p>
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center space-x-3">
            <div className={`p-2 rounded-lg ${walletInfo.connected ? 'bg-green-500/10' : 'bg-red-500/10'}`}>
              <Wallet className={`h-6 w-6 ${walletInfo.connected ? 'text-green-500' : 'text-red-500'}`} />
            </div>
            <div>
              <h3 className="text-lg font-semibold text-white">Кошелек</h3>
              <p className={`text-sm ${walletInfo.connected ? 'text-green-400' : 'text-red-400'}`}>
                {walletInfo.connected ? 'Подключен' : 'Отключен'}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Статистические карточки */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {statCards.map((card, index) => {
          const Icon = card.icon;
          return (
            <div key={index} className="card">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-slate-400">{card.title}</p>
                  <p className="text-2xl font-bold text-white mt-1">{card.value}</p>
                </div>
                <div className={`p-3 rounded-lg ${card.bgColor}`}>
                  <Icon className={`h-6 w-6 ${card.color}`} />
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Последние транзакции */}
      {walletInfo.connected && (
        <div className="card">
          <div className="card-header">
            <h3 className="card-title">Последние транзакции</h3>
            <p className="card-subtitle">История операций в блокчейне</p>
          </div>
          <div className="space-y-3">
            {recentTransactions.length > 0 ? (
              recentTransactions.map((tx, index) => (
                <div key={index} className="flex items-center justify-between p-3 bg-slate-700 rounded-lg">
                  <div className="flex items-center space-x-3">
                    <div className="w-2 h-2 bg-green-400 rounded-full"></div>
                    <div>
                      <p className="text-sm font-mono text-slate-300">
                        {tx.signature.slice(0, 8)}...{tx.signature.slice(-8)}
                      </p>
                      <p className="text-xs text-slate-400">
                        {new Date(tx.blockTime * 1000).toLocaleString()}
                      </p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="text-sm text-green-400">Подтверждено</p>
                    <p className="text-xs text-slate-400">Slot: {tx.slot}</p>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8">
                <Activity className="h-12 w-12 text-slate-500 mx-auto mb-4" />
                <p className="text-slate-400">Нет транзакций</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Быстрые действия */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Быстрые действия</h3>
          <p className="card-subtitle">Основные операции W3B2 протокола</p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <button className="p-4 bg-slate-700 hover:bg-slate-600 rounded-lg text-left transition-colors">
            <CreditCard className="h-8 w-8 text-blue-500 mb-2" />
            <h4 className="font-semibold text-white">Запрос финансирования</h4>
            <p className="text-sm text-slate-400">Подать заявку на пополнение</p>
          </button>
          
          <button className="p-4 bg-slate-700 hover:bg-slate-600 rounded-lg text-left transition-colors">
            <Database className="h-8 w-8 text-purple-500 mb-2" />
            <h4 className="font-semibold text-white">Управление записями</h4>
            <p className="text-sm text-slate-400">CRUD операции с данными</p>
          </button>
          
          <button className="p-4 bg-slate-700 hover:bg-slate-600 rounded-lg text-left transition-colors">
            <Users className="h-8 w-8 text-green-500 mb-2" />
            <h4 className="font-semibold text-white">Управление сессиями</h4>
            <p className="text-sm text-slate-400">Создание и закрытие сессий</p>
          </button>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
