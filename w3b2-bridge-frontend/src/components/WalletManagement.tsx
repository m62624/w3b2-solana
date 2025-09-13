import React, { useState, useEffect } from 'react';
import { 
  Wallet, 
  Plus, 
  Users, 
  Shield, 
  RefreshCw,
  Copy,
  Download,
  Key,
  Coins,
  X
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';
import { Keypair, PublicKey } from '@solana/web3.js';
import toast from 'react-hot-toast';

interface AdminAccount {
  public_key: string;
  co_signer: string;
  is_registered: boolean;
  funding_amount: number;
  created_at: number;
  last_activity: number;
}

const WalletManagement: React.FC = () => {
  const { walletInfo, initializeWallet, getPrivateKey } = useWalletContext();
  const [admins, setAdmins] = useState<AdminAccount[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [showAdminForm, setShowAdminForm] = useState(false);
  const [generatedWallets, setGeneratedWallets] = useState<Keypair[]>([]);
  
  // Форма регистрации администратора
  const [adminForm, setAdminForm] = useState({
    authority: '',
    coSigner: '',
    fundingAmount: '',
  });

  useEffect(() => {
    loadAdmins();
  }, []);

  const loadAdmins = async () => {
    try {
      setIsLoading(true);
      const { apiService } = await import('../services/apiService');
      const response = await apiService.getAdmins();
      if (response.success && response.data) {
        setAdmins(response.data);
      }
    } catch (error) {
      console.error('Ошибка загрузки администраторов:', error);
      toast.error('Ошибка загрузки администраторов');
    } finally {
      setIsLoading(false);
    }
  };

  const generateNewWallet = () => {
    const newWallet = Keypair.generate();
    setGeneratedWallets(prev => [...prev, newWallet]);
    toast.success('Новый кошелек сгенерирован');
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    toast.success(`${label} скопирован в буфер обмена`);
  };

  const exportWallet = (wallet: Keypair) => {
    const walletData = {
      publicKey: wallet.publicKey.toBase58(),
      privateKey: Buffer.from(wallet.secretKey).toString('base64'),
      timestamp: Date.now()
    };
    
    const blob = new Blob([JSON.stringify(walletData, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `wallet_${wallet.publicKey.toBase58().slice(0, 8)}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    
    toast.success('Кошелек экспортирован');
  };

  const handleRegisterAdmin = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!adminForm.authority || !adminForm.coSigner || !adminForm.fundingAmount) {
      toast.error('Заполните все поля');
      return;
    }

    try {
      setIsLoading(true);
      const { apiService } = await import('../services/apiService');
      const response = await apiService.registerAdmin(
        adminForm.authority,
        adminForm.coSigner,
        parseInt(adminForm.fundingAmount)
      );
      
      if (response.success) {
        toast.success('Администратор зарегистрирован успешно!');
        setAdminForm({ authority: '', coSigner: '', fundingAmount: '' });
        setShowAdminForm(false);
        await loadAdmins();
      } else {
        toast.error(response.error || 'Ошибка регистрации администратора');
      }
    } catch (error) {
      console.error('Ошибка регистрации администратора:', error);
      toast.error('Ошибка регистрации администратора');
    } finally {
      setIsLoading(false);
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp).toLocaleString('ru-RU');
  };

  return (
    <div className="space-y-6">
      {/* Заголовок */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Управление кошельками</h1>
          <p className="text-slate-400 mt-1">
            Создание кошельков и регистрация администраторов
          </p>
        </div>
        <div className="flex space-x-3">
          <button
            onClick={loadAdmins}
            disabled={isLoading}
            className="btn-outline flex items-center space-x-2"
          >
            <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
            <span>Обновить</span>
          </button>
        </div>
      </div>

      {/* Форма регистрации администратора */}
      <div className="card border-2 border-blue-500/20 bg-gradient-to-br from-blue-900/10 to-purple-900/10">
        <div className="card-header">
          <div className="flex items-center justify-between">
            <h3 className="card-title flex items-center space-x-2 text-blue-300">
              <Shield className="h-6 w-6" />
              <span>Регистрация администратора</span>
            </h3>
            <div className="flex items-center space-x-2">
              <div className="px-3 py-1 bg-blue-500/20 rounded-full text-xs text-blue-300">
                Blockchain
              </div>
              <button
                onClick={() => setShowAdminForm(!showAdminForm)}
                className={`p-2 rounded-lg transition-all duration-200 ${
                  showAdminForm 
                    ? 'bg-red-500/20 text-red-300 hover:bg-red-500/30' 
                    : 'bg-blue-500/20 text-blue-300 hover:bg-blue-500/30'
                }`}
              >
                {showAdminForm ? <X className="h-4 w-4" /> : <Plus className="h-4 w-4" />}
              </button>
            </div>
          </div>
        </div>
        
        {showAdminForm && (
          <div className="p-6">
            <div className="mb-6 p-4 bg-blue-500/10 rounded-lg border border-blue-500/20">
              <h4 className="text-sm font-semibold text-blue-300 mb-2">Информация о регистрации</h4>
              <p className="text-sm text-slate-400">
                Администратор будет зарегистрирован в блокчейне Solana с указанной суммой финансирования. 
                Убедитесь, что у authority кошелька достаточно SOL для комиссии.
              </p>
            </div>

            <form onSubmit={handleRegisterAdmin} className="space-y-8">
              <div className="input-group-grid">
                <div className="input-group">
                  <label className="input-label-required">
                    Публичный ключ администратора
                  </label>
                  <div className="input-container">
                    <input
                      type="text"
                      value={adminForm.authority}
                      onChange={(e) => setAdminForm({ ...adminForm, authority: e.target.value })}
                      className="admin-input input-with-icon"
                      placeholder="Введите публичный ключ администратора"
                      required
                    />
                    <div className="input-icon">
                      <Key className="h-4 w-4" />
                    </div>
                  </div>
                  <p className="input-hint">Кто будет управлять административными функциями</p>
                </div>

                <div className="input-group">
                  <label className="input-label-required">
                    Co-signer ключ
                  </label>
                  <div className="input-container">
                    <input
                      type="text"
                      value={adminForm.coSigner}
                      onChange={(e) => setAdminForm({ ...adminForm, coSigner: e.target.value })}
                      className="admin-input input-with-icon"
                      placeholder="Введите co-signer ключ"
                      required
                    />
                    <div className="input-icon">
                      <Shield className="h-4 w-4" />
                    </div>
                  </div>
                  <p className="input-hint">Уникальный ключ для PDA (можно сгенерировать новый)</p>
                </div>
              </div>

              <div className="input-group">
                <label className="input-label-required">
                  Сумма финансирования
                </label>
                <div className="input-container">
                  <input
                    type="number"
                    value={adminForm.fundingAmount}
                    onChange={(e) => setAdminForm({ ...adminForm, fundingAmount: e.target.value })}
                    className="admin-input input-with-icon"
                    placeholder="1000000000"
                    min="0"
                    step="1000000"
                    required
                  />
                  <div className="input-icon">
                    <Coins className="h-4 w-4" />
                  </div>
                </div>
                <div className="flex items-center justify-between text-xs text-slate-500 mt-2">
                  <span>1 SOL = 1,000,000,000 lamports</span>
                  <span className="text-blue-400 font-medium">
                    ≈ {(parseInt(adminForm.fundingAmount || '0') / 1000000000).toFixed(4)} SOL
                  </span>
                </div>
              </div>

              <div className="flex items-center justify-between pt-4 border-t border-slate-700">
                <div className="flex items-center space-x-2 text-sm text-slate-400">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  <span>Готово к регистрации</span>
                </div>
                <div className="flex space-x-3">
                  <button
                    type="button"
                    onClick={() => setShowAdminForm(false)}
                    className="px-4 py-2 text-sm font-medium text-slate-300 bg-slate-700 hover:bg-slate-600 rounded-lg transition-colors duration-200"
                  >
                    Отмена
                  </button>
                  <button
                    type="submit"
                    disabled={isLoading}
                    className="px-6 py-2 text-sm font-medium text-white bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-700 hover:to-purple-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-all duration-200 flex items-center space-x-2"
                  >
                    {isLoading ? (
                      <RefreshCw className="h-4 w-4 animate-spin" />
                    ) : (
                      <Shield className="h-4 w-4" />
                    )}
                    <span>Зарегистрировать администратора</span>
                  </button>
                </div>
              </div>
            </form>
          </div>
        )}
      </div>

      {/* Текущий кошелек */}
      {walletInfo.connected && (
        <div className="card">
          <div className="card-header">
            <h3 className="card-title flex items-center space-x-2">
              <Wallet className="h-5 w-5" />
              <span>Текущий кошелек</span>
            </h3>
          </div>
          <div className="p-6">
            <div className="space-y-4">
              <div className="input-group">
                <label className="input-label">Публичный ключ</label>
                <div className="flex items-center space-x-2">
                  <code className="flex-1 p-3 bg-slate-800/50 border border-slate-600 rounded-lg text-sm text-slate-300 break-all font-mono">
                    {walletInfo.publicKey?.toBase58()}
                  </code>
                  <button
                    onClick={() => copyToClipboard(walletInfo.publicKey?.toBase58() || '', 'Публичный ключ')}
                    className="btn-outline p-3 hover:bg-slate-700 transition-colors duration-200"
                  >
                    <Copy className="h-4 w-4" />
                  </button>
                </div>
              </div>
              <div className="input-group">
                <label className="input-label">Приватный ключ (Base64)</label>
                <div className="flex items-center space-x-2">
                  <code className="flex-1 p-3 bg-slate-800/50 border border-slate-600 rounded-lg text-sm text-slate-300 break-all font-mono">
                    {getPrivateKey() || 'Не доступен'}
                  </code>
                  <button
                    onClick={() => copyToClipboard(getPrivateKey() || '', 'Приватный ключ')}
                    className="btn-outline p-3 hover:bg-slate-700 transition-colors duration-200"
                  >
                    <Copy className="h-4 w-4" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Генерация кошельков */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title flex items-center space-x-2">
            <Key className="h-5 w-5" />
            <span>Генерация кошельков</span>
          </h3>
        </div>
        <div className="p-6">
          <div className="flex items-center justify-between mb-4">
            <p className="text-slate-400">
              Создавайте новые кошельки для тестирования
            </p>
            <button
              onClick={generateNewWallet}
              className="btn-primary flex items-center space-x-2"
            >
              <Plus className="h-4 w-4" />
              <span>Создать кошелек</span>
            </button>
          </div>
          
          {generatedWallets.length > 0 && (
            <div className="space-y-4">
              <h4 className="text-lg font-semibold text-white">Сгенерированные кошельки</h4>
              {generatedWallets.map((wallet, index) => (
                <div key={index} className="p-4 bg-slate-800 rounded-lg">
                  <div className="space-y-4">
                    <div className="input-group">
                      <label className="input-label">Публичный ключ</label>
                      <div className="flex items-center space-x-2">
                        <code className="flex-1 p-3 bg-slate-900/50 border border-slate-700 rounded-lg text-sm text-slate-300 break-all font-mono">
                          {wallet.publicKey.toBase58()}
                        </code>
                        <button
                          onClick={() => copyToClipboard(wallet.publicKey.toBase58(), 'Публичный ключ')}
                          className="btn-outline p-3 hover:bg-slate-700 transition-colors duration-200"
                        >
                          <Copy className="h-4 w-4" />
                        </button>
                      </div>
                    </div>
                    <div className="input-group">
                      <label className="input-label">Приватный ключ (Base64)</label>
                      <div className="flex items-center space-x-2">
                        <code className="flex-1 p-3 bg-slate-900/50 border border-slate-700 rounded-lg text-sm text-slate-300 break-all font-mono">
                          {Buffer.from(wallet.secretKey).toString('base64')}
                        </code>
                        <button
                          onClick={() => copyToClipboard(Buffer.from(wallet.secretKey).toString('base64'), 'Приватный ключ')}
                          className="btn-outline p-3 hover:bg-slate-700 transition-colors duration-200"
                        >
                          <Copy className="h-4 w-4" />
                        </button>
                      </div>
                    </div>
                    <div className="flex space-x-2">
                      <button
                        onClick={() => exportWallet(wallet)}
                        className="btn-outline flex items-center space-x-2"
                      >
                        <Download className="h-4 w-4" />
                        <span>Экспорт</span>
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Список администраторов */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title flex items-center space-x-2">
            <Users className="h-5 w-5" />
            <span>Зарегистрированные администраторы</span>
          </h3>
        </div>
        <div className="p-6">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <RefreshCw className="h-6 w-6 animate-spin text-blue-500" />
              <span className="ml-2 text-slate-400">Загрузка...</span>
            </div>
          ) : admins.length === 0 ? (
            <div className="text-center py-8">
              <Users className="h-12 w-12 text-slate-600 mx-auto mb-4" />
              <p className="text-slate-400">Администраторы не найдены</p>
            </div>
          ) : (
            <div className="space-y-4">
              {admins.map((admin, index) => (
                <div key={index} className="p-4 bg-slate-800 rounded-lg">
                  <div className="space-y-3">
                    <div className="flex items-center justify-between">
                      <h4 className="text-lg font-semibold text-white">
                        Администратор #{index + 1}
                      </h4>
                      <div className="flex items-center space-x-2">
                        <Coins className="h-4 w-4 text-yellow-500" />
                        <span className="text-sm text-slate-400">
                          {(admin.funding_amount / 1000000000).toFixed(4)} SOL
                        </span>
                      </div>
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                      <div className="input-group">
                        <label className="input-label">Публичный ключ</label>
                        <div className="flex items-center space-x-2">
                          <code className="flex-1 p-3 bg-slate-900/50 border border-slate-700 rounded-lg text-sm text-slate-300 break-all font-mono">
                            {admin.public_key}
                          </code>
                          <button
                            onClick={() => copyToClipboard(admin.public_key, 'Публичный ключ')}
                            className="btn-outline p-3 hover:bg-slate-700 transition-colors duration-200"
                          >
                            <Copy className="h-4 w-4" />
                          </button>
                        </div>
                      </div>
                      <div className="input-group">
                        <label className="input-label">Co-signer</label>
                        <div className="flex items-center space-x-2">
                          <code className="flex-1 p-3 bg-slate-900/50 border border-slate-700 rounded-lg text-sm text-slate-300 break-all font-mono">
                            {admin.co_signer}
                          </code>
                          <button
                            onClick={() => copyToClipboard(admin.co_signer, 'Co-signer')}
                            className="btn-outline p-3 hover:bg-slate-700 transition-colors duration-200"
                          >
                            <Copy className="h-4 w-4" />
                          </button>
                        </div>
                      </div>
                    </div>
                    <div className="text-sm text-slate-400">
                      <p>Создан: {formatDate(admin.created_at)}</p>
                      <p>Последняя активность: {formatDate(admin.last_activity)}</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default WalletManagement;
