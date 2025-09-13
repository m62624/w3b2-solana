import React, { useState, useEffect } from 'react';
import { 
  Database, 
  Plus, 
  Edit, 
  Trash2, 
  Search, 
  Filter,
  RefreshCw,
  Eye,
  Calendar,
  User
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';
import { useApiContext } from '../contexts/ApiContext';
import { DatabaseRecord, CrudOperation } from '../types/index.js';
import toast from 'react-hot-toast';

const Records: React.FC = () => {
  const { walletInfo } = useWalletContext();
  const { 
    getUserRecords, 
    createRecord, 
    updateRecord, 
    deleteRecord,
    isLoading 
  } = useApiContext();
  
  const [records, setRecords] = useState<DatabaseRecord[]>([]);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingRecord, setEditingRecord] = useState<DatabaseRecord | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [filterStatus, setFilterStatus] = useState<'all' | 'recent' | 'old'>('all');

  // Форма создания/редактирования записи
  const [recordForm, setRecordForm] = useState({
    title: '',
    description: '',
    data: '',
  });

  useEffect(() => {
    if (walletInfo.connected && walletInfo.publicKey) {
      loadRecords();
    }
  }, [walletInfo.connected, walletInfo.publicKey]);

  const loadRecords = async () => {
    if (!walletInfo.publicKey) return;

    try {
      const response = await getUserRecords(walletInfo.publicKey.toBase58());
      setRecords(response);
    } catch (error) {
      console.error('Ошибка загрузки записей:', error);
      toast.error('Ошибка загрузки записей');
    }
  };

  const handleCreateRecord = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!walletInfo.publicKey) {
      toast.error('Кошелек не подключен');
      return;
    }

    if (!recordForm.title.trim()) {
      toast.error('Введите название записи');
      return;
    }

    try {
      const recordData = {
        title: recordForm.title,
        description: recordForm.description,
        data: recordForm.data,
        created_at: Date.now(),
      };

      await createRecord(walletInfo.publicKey.toBase58(), recordData);
      toast.success('Запись создана');
      setRecordForm({ title: '', description: '', data: '' });
      setShowCreateForm(false);
      await loadRecords();
    } catch (error) {
      toast.error('Ошибка создания записи');
    }
  };

  const handleUpdateRecord = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!walletInfo.publicKey || !editingRecord) return;

    try {
      const updatedData = {
        title: recordForm.title,
        description: recordForm.description,
        data: recordForm.data,
        updated_at: Date.now(),
      };

      await updateRecord(editingRecord.id, updatedData, walletInfo.publicKey.toBase58());
      toast.success('Запись обновлена');
      setEditingRecord(null);
      setRecordForm({ title: '', description: '', data: '' });
      await loadRecords();
    } catch (error) {
      toast.error('Ошибка обновления записи');
    }
  };

  const handleDeleteRecord = async (recordId: string) => {
    if (!walletInfo.publicKey) return;

    if (!window.confirm('Вы уверены, что хотите удалить эту запись?')) {
      return;
    }

    try {
      await deleteRecord(recordId, walletInfo.publicKey.toBase58());
      toast.success('Запись удалена');
      await loadRecords();
    } catch (error) {
      toast.error('Ошибка удаления записи');
    }
  };

  const handleEditRecord = (record: DatabaseRecord) => {
    setEditingRecord(record);
    setRecordForm({
      title: record.data.title || '',
      description: record.data.description || '',
      data: record.data.data || '',
    });
    setShowCreateForm(true);
  };

  const handleCancelEdit = () => {
    setEditingRecord(null);
    setRecordForm({ title: '', description: '', data: '' });
    setShowCreateForm(false);
  };

  const filteredRecords = records.filter(record => {
    const matchesSearch = record.data.title?.toLowerCase().includes(searchTerm.toLowerCase()) ||
                         record.data.description?.toLowerCase().includes(searchTerm.toLowerCase());
    
    const now = Date.now();
    const recordAge = now - record.created_at;
    const oneDay = 24 * 60 * 60 * 1000;
    
    let matchesFilter = true;
    if (filterStatus === 'recent') {
      matchesFilter = recordAge < oneDay;
    } else if (filterStatus === 'old') {
      matchesFilter = recordAge >= oneDay;
    }
    
    return matchesSearch && matchesFilter;
  });

  if (!walletInfo.connected) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold text-white">Записи</h1>
          <p className="text-slate-400 mt-1">Управление данными в базе</p>
        </div>
        
        <div className="card">
          <div className="text-center py-8">
            <Database className="h-16 w-16 text-slate-500 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Кошелек не подключен</h3>
            <p className="text-slate-400">Подключите кошелек для работы с записями</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Заголовок */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Записи</h1>
          <p className="text-slate-400 mt-1">
            Управление данными в базе W3B2
          </p>
        </div>
        <div className="flex space-x-3">
          <button
            onClick={loadRecords}
            disabled={isLoading}
            className="btn-outline flex items-center space-x-2"
          >
            <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
            <span>Обновить</span>
          </button>
          <button
            onClick={() => setShowCreateForm(true)}
            className="btn-primary flex items-center space-x-2"
          >
            <Plus className="h-4 w-4" />
            <span>Новая запись</span>
          </button>
        </div>
      </div>

      {/* Фильтры и поиск */}
      <div className="card">
        <div className="flex flex-col sm:flex-row gap-4">
          <div className="flex-1">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-slate-400" />
              <input
                type="text"
                placeholder="Поиск записей..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="form-input pl-10"
              />
            </div>
          </div>
          <div className="flex space-x-2">
            <select
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value as any)}
              className="form-select"
            >
              <option value="all">Все записи</option>
              <option value="recent">Недавние</option>
              <option value="old">Старые</option>
            </select>
          </div>
        </div>
      </div>

      {/* Форма создания/редактирования */}
      {showCreateForm && (
        <div className="card">
          <div className="card-header">
            <h3 className="card-title">
              {editingRecord ? 'Редактировать запись' : 'Создать запись'}
            </h3>
            <p className="card-subtitle">
              {editingRecord ? 'Внесите изменения в запись' : 'Добавьте новую запись в базу данных'}
            </p>
          </div>
          
          <form onSubmit={editingRecord ? handleUpdateRecord : handleCreateRecord} className="space-y-4">
            <div className="form-group">
              <label className="form-label">Название *</label>
              <input
                type="text"
                value={recordForm.title}
                onChange={(e) => setRecordForm({ ...recordForm, title: e.target.value })}
                className="form-input"
                placeholder="Введите название записи"
                required
              />
            </div>
            
            <div className="form-group">
              <label className="form-label">Описание</label>
              <textarea
                value={recordForm.description}
                onChange={(e) => setRecordForm({ ...recordForm, description: e.target.value })}
                className="form-textarea"
                placeholder="Описание записи"
                rows={3}
              />
            </div>
            
            <div className="form-group">
              <label className="form-label">Данные (JSON)</label>
              <textarea
                value={recordForm.data}
                onChange={(e) => setRecordForm({ ...recordForm, data: e.target.value })}
                className="form-textarea font-mono text-sm"
                placeholder='{"key": "value"}'
                rows={4}
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
                  <Database className="h-4 w-4" />
                )}
                <span>{isLoading ? 'Сохранение...' : (editingRecord ? 'Обновить' : 'Создать')}</span>
              </button>
              <button
                type="button"
                onClick={handleCancelEdit}
                className="btn-outline"
              >
                Отмена
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Список записей */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Мои записи</h3>
          <p className="card-subtitle">
            {filteredRecords.length} из {records.length} записей
          </p>
        </div>
        
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="loading-spinner"></div>
            <span className="ml-2 text-slate-400">Загрузка...</span>
          </div>
        ) : filteredRecords.length > 0 ? (
          <div className="space-y-4">
            {filteredRecords.map((record) => (
              <div key={record.id} className="p-4 bg-slate-700 rounded-lg">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <h4 className="text-lg font-semibold text-white mb-2">
                      {record.data.title || 'Без названия'}
                    </h4>
                    {record.data.description && (
                      <p className="text-slate-300 mb-3">{record.data.description}</p>
                    )}
                    <div className="flex items-center space-x-4 text-sm text-slate-400">
                      <div className="flex items-center space-x-1">
                        <Calendar className="h-4 w-4" />
                        <span>{new Date(record.created_at).toLocaleDateString()}</span>
                      </div>
                      <div className="flex items-center space-x-1">
                        <User className="h-4 w-4" />
                        <span className="font-mono">
                          {typeof record.owner === 'string' 
                            ? `${record.owner.slice(0, 8)}...${record.owner.slice(-8)}`
                            : `${record.owner.toBase58().slice(0, 8)}...${record.owner.toBase58().slice(-8)}`
                          }
                        </span>
                      </div>
                    </div>
                    {record.data.data && (
                      <div className="mt-3 p-3 bg-slate-800 rounded border-l-4 border-blue-500">
                        <pre className="text-xs text-slate-300 overflow-x-auto">
                          {JSON.stringify(JSON.parse(record.data.data), null, 2)}
                        </pre>
                      </div>
                    )}
                  </div>
                  <div className="flex space-x-2 ml-4">
                    <button
                      onClick={() => handleEditRecord(record)}
                      className="btn-outline p-2"
                      title="Редактировать"
                    >
                      <Edit className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => handleDeleteRecord(record.id)}
                      className="btn-danger p-2"
                      title="Удалить"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Database className="h-12 w-12 text-slate-500 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Нет записей</h3>
            <p className="text-slate-400 mb-4">
              {searchTerm || filterStatus !== 'all' 
                ? 'Записи не найдены по заданным критериям'
                : 'Создайте первую запись для начала работы'
              }
            </p>
            {!searchTerm && filterStatus === 'all' && (
              <button
                onClick={() => setShowCreateForm(true)}
                className="btn-primary flex items-center space-x-2 mx-auto"
              >
                <Plus className="h-4 w-4" />
                <span>Создать запись</span>
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default Records;
