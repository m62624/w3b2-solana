import React, { useState, useEffect } from 'react';
import { 
  CreditCard, 
  Plus, 
  CheckCircle, 
  XCircle, 
  Clock, 
  RefreshCw,
  DollarSign,
  User,
  Calendar
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';
import { useApiContext } from '../contexts/ApiContext';
import { FundingRequest, FundingStatus } from '../types/index.js';
import toast from 'react-hot-toast';

const Funding: React.FC = () => {
  const { walletInfo, balance } = useWalletContext();
  const { requestFunding, getFundingRequests, approveFunding } = useApiContext();
  
  const [fundingRequests, setFundingRequests] = useState<FundingRequest[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [showRequestForm, setShowRequestForm] = useState(false);
  
  // Форма запроса на финансирование
  const [requestForm, setRequestForm] = useState({
    amount: '',
    targetAdmin: '',
  });

  useEffect(() => {
    loadFundingRequests();
  }, []);

  const loadFundingRequests = async () => {
    try {
      setIsLoading(true);
      const response = await getFundingRequests();
      if (response.success) {
        setFundingRequests(response.data);
      }
    } catch (error) {
      console.error('Ошибка загрузки запросов:', error);
      toast.error('Ошибка загрузки запросов на финансирование');
    } finally {
      setIsLoading(false);
    }
  };

  const handleRequestFunding = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!walletInfo.publicKey) {
      toast.error('Кошелек не подключен');
      return;
    }

    if (!requestForm.amount || !requestForm.targetAdmin) {
      toast.error('Заполните все поля');
      return;
    }

    try {
      setIsLoading(true);
      const response = await requestFunding(
        walletInfo.publicKey.toBase58(),
        parseFloat(requestForm.amount),
        requestForm.targetAdmin
      );

      if (response.success) {
        toast.success('Запрос на финансирование отправлен');
        setRequestForm({ amount: '', targetAdmin: '' });
        setShowRequestForm(false);
        await loadFundingRequests();
      } else {
        toast.error(response.error || 'Ошибка отправки запроса');
      }
    } catch (error) {
      toast.error('Ошибка отправки запроса на финансирование');
    } finally {
      setIsLoading(false);
    }
  };

  const handleApproveFunding = async (requestId: string) => {
    try {
      setIsLoading(true);
      const response = await approveFunding(requestId);
      
      if (response.success) {
        toast.success('Финансирование одобрено');
        await loadFundingRequests();
      } else {
        toast.error(response.error || 'Ошибка одобрения');
      }
    } catch (error) {
      toast.error('Ошибка одобрения финансирования');
    } finally {
      setIsLoading(false);
    }
  };

  const getStatusIcon = (status: FundingStatus) => {
    switch (status) {
      case FundingStatus.Pending:
        return <Clock className="h-4 w-4 text-yellow-500" />;
      case FundingStatus.Approved:
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case FundingStatus.Rejected:
        return <XCircle className="h-4 w-4 text-red-500" />;
      default:
        return <Clock className="h-4 w-4 text-gray-500" />;
    }
  };

  const getStatusText = (status: FundingStatus) => {
    switch (status) {
      case FundingStatus.Pending:
        return 'Ожидает';
      case FundingStatus.Approved:
        return 'Одобрено';
      case FundingStatus.Rejected:
        return 'Отклонено';
      default:
        return 'Неизвестно';
    }
  };

  const getStatusColor = (status: FundingStatus) => {
    switch (status) {
      case FundingStatus.Pending:
        return 'status-pending';
      case FundingStatus.Approved:
        return 'status-approved';
      case FundingStatus.Rejected:
        return 'status-rejected';
      default:
        return 'status-pending';
    }
  };

  return (
    <div className="space-y-6">
      {/* Заголовок */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Финансирование</h1>
          <p className="text-slate-400 mt-1">
            Управление запросами на пополнение баланса
          </p>
        </div>
        <div className="flex space-x-3">
          <button
            onClick={loadFundingRequests}
            disabled={isLoading}
            className="btn-outline flex items-center space-x-2"
          >
            <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
            <span>Обновить</span>
          </button>
          {walletInfo.connected && (
            <button
              onClick={() => setShowRequestForm(true)}
              className="btn-primary flex items-center space-x-2"
            >
              <Plus className="h-4 w-4" />
              <span>Новый запрос</span>
            </button>
          )}
        </div>
      </div>

      {/* Информация о балансе */}
      {walletInfo.connected && (
        <div className="card">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <div className="p-3 bg-blue-500/10 rounded-lg">
                <DollarSign className="h-6 w-6 text-blue-500" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-white">Текущий баланс</h3>
                <p className="text-2xl font-bold text-blue-500">{balance.toFixed(4)} SOL</p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Форма запроса на финансирование */}
      {showRequestForm && (
        <div className="card">
          <div className="card-header">
            <h3 className="card-title">Запрос на финансирование</h3>
            <p className="card-subtitle">Подайте заявку на пополнение баланса</p>
          </div>
          
          <form onSubmit={handleRequestFunding} className="space-y-4">
            <div className="form-group">
              <label className="form-label">Сумма (SOL)</label>
              <input
                type="number"
                step="0.001"
                min="0"
                value={requestForm.amount}
                onChange={(e) => setRequestForm({ ...requestForm, amount: e.target.value })}
                className="form-input"
                placeholder="0.000"
                required
              />
            </div>
            
            <div className="form-group">
              <label className="form-label">Администратор</label>
              <input
                type="text"
                value={requestForm.targetAdmin}
                onChange={(e) => setRequestForm({ ...requestForm, targetAdmin: e.target.value })}
                className="form-input font-mono"
                placeholder="Публичный ключ администратора"
                required
              />
            </div>
            
            <div className="flex space-x-3">
              <button
                type="submit"
                disabled={isLoading}
                className="btn-primary flex items-center space-x-2"
              >
                {isLoading ? (
                  <div className="loading-spinner"></div>
                ) : (
                  <CreditCard className="h-4 w-4" />
                )}
                <span>{isLoading ? 'Отправка...' : 'Отправить запрос'}</span>
              </button>
              <button
                type="button"
                onClick={() => setShowRequestForm(false)}
                className="btn-outline"
              >
                Отмена
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Список запросов на финансирование */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Запросы на финансирование</h3>
          <p className="card-subtitle">История всех запросов</p>
        </div>
        
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="loading-spinner"></div>
            <span className="ml-2 text-slate-400">Загрузка...</span>
          </div>
        ) : fundingRequests.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="table">
              <thead>
                <tr>
                  <th>ID</th>
                  <th>Пользователь</th>
                  <th>Сумма</th>
                  <th>Статус</th>
                  <th>Дата создания</th>
                  <th>Действия</th>
                </tr>
              </thead>
              <tbody>
                {fundingRequests.map((request) => (
                  <tr key={request.id}>
                    <td className="font-mono text-sm">
                      {request.id?.slice(0, 8)}...
                    </td>
                    <td className="font-mono text-sm">
                      {typeof request.user_wallet === 'string' 
                        ? `${request.user_wallet.slice(0, 8)}...${request.user_wallet.slice(-8)}`
                        : `${request.user_wallet.toBase58().slice(0, 8)}...${request.user_wallet.toBase58().slice(-8)}`
                      }
                    </td>
                    <td className="font-semibold">
                      {request.amount} SOL
                    </td>
                    <td>
                      <div className="flex items-center space-x-2">
                        {getStatusIcon(request.status)}
                        <span className={getStatusColor(request.status)}>
                          {getStatusText(request.status)}
                        </span>
                      </div>
                    </td>
                    <td className="text-sm text-slate-400">
                      {new Date(request.created_at).toLocaleString()}
                    </td>
                    <td>
                      {request.status === FundingStatus.Pending && (
                        <button
                          onClick={() => request.id && handleApproveFunding(request.id)}
                          disabled={isLoading}
                          className="btn-primary text-xs py-1 px-2"
                        >
                          Одобрить
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="text-center py-8">
            <CreditCard className="h-12 w-12 text-slate-500 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Нет запросов</h3>
            <p className="text-slate-400">Запросы на финансирование будут отображаться здесь</p>
          </div>
        )}
      </div>

      {/* Статистика */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="card">
          <div className="flex items-center space-x-3">
            <Clock className="h-8 w-8 text-yellow-500" />
            <div>
              <p className="text-sm text-slate-400">Ожидают</p>
              <p className="text-2xl font-bold text-white">
                {fundingRequests.filter(r => r.status === FundingStatus.Pending).length}
              </p>
            </div>
          </div>
        </div>
        
        <div className="card">
          <div className="flex items-center space-x-3">
            <CheckCircle className="h-8 w-8 text-green-500" />
            <div>
              <p className="text-sm text-slate-400">Одобрено</p>
              <p className="text-2xl font-bold text-white">
                {fundingRequests.filter(r => r.status === FundingStatus.Approved).length}
              </p>
            </div>
          </div>
        </div>
        
        <div className="card">
          <div className="flex items-center space-x-3">
            <XCircle className="h-8 w-8 text-red-500" />
            <div>
              <p className="text-sm text-slate-400">Отклонено</p>
              <p className="text-2xl font-bold text-white">
                {fundingRequests.filter(r => r.status === FundingStatus.Rejected).length}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Funding;
