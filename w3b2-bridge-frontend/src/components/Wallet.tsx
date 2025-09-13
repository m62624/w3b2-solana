import React, { useState, useEffect, useCallback } from 'react';
import { 
  Wallet as WalletIcon, 
  Download, 
  Upload, 
  Copy, 
  RefreshCw, 
  Eye, 
  EyeOff,
  Key,
  QrCode,
  ExternalLink,
  Shield,
  RotateCcw
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';
import { useApiContext } from '../contexts/ApiContext';
import { MnemonicService, type MnemonicData } from '../services/mnemonicService';
import toast from 'react-hot-toast';

const Wallet: React.FC = () => {
  const { 
    walletInfo, 
    balance, 
    isLoading, 
    generateWallet, 
    importWallet, 
    disconnect, 
    refreshBalance,
    exportWallet,
    getPrivateKey,
    getRecentTransactions
  } = useWalletContext();
  
  const { registerUser } = useApiContext();
  
  const [showPrivateKey, setShowPrivateKey] = useState(false);
  const [importKey, setImportKey] = useState('');
  const [recentTransactions, setRecentTransactions] = useState<{ signature?: string; blockTime?: number; slot?: number }[]>([]);
  const [isRegistering, setIsRegistering] = useState(false);
  
  // Состояние для мнемонических фраз
  const [mnemonicData, setMnemonicData] = useState<MnemonicData | null>(null);
  const [showMnemonic, setShowMnemonic] = useState(false);
  const [importMnemonic, setImportMnemonic] = useState('');
  const [mnemonicWords, setMnemonicWords] = useState<string[]>([]);
  const [isGeneratingMnemonic, setIsGeneratingMnemonic] = useState(false);
  const [isRestoringMnemonic, setIsRestoringMnemonic] = useState(false);

  const loadRecentTransactions = useCallback(async () => {
    try {
      const transactions = await getRecentTransactions(10);
      setRecentTransactions(transactions);
    } catch {
      // Игнорируем ошибки загрузки транзакций
    }
  }, [getRecentTransactions]);

  useEffect(() => {
    if (walletInfo.connected) {
      loadRecentTransactions();
    }
  }, [walletInfo.connected, loadRecentTransactions]);

  // Функции для работы с мнемоническими фразами
  const generateMnemonic = async () => {
    try {
      setIsGeneratingMnemonic(true);
      const data = MnemonicService.generateMnemonic();
      setMnemonicData(data);
      setMnemonicWords(MnemonicService.formatMnemonic(data.mnemonic));
      setShowMnemonic(true);
      toast.success('Мнемоническая фраза сгенерирована!');
    } catch {
      toast.error('Ошибка генерации мнемонической фразы');
    } finally {
      setIsGeneratingMnemonic(false);
    }
  };

  const restoreFromMnemonic = async () => {
    try {
      setIsRestoringMnemonic(true);
      const data = MnemonicService.restoreFromMnemonic(importMnemonic);
      setMnemonicData(data);
      setMnemonicWords(MnemonicService.formatMnemonic(data.mnemonic));
      setShowMnemonic(true);
      toast.success('Ключи восстановлены из мнемонической фразы!');
    } catch {
      toast.error('Неверная мнемоническая фраза');
    } finally {
      setIsRestoringMnemonic(false);
    }
  };

  const useMnemonicWallet = async () => {
    if (!mnemonicData) return;
    
    try {
      const keypair = MnemonicService.createKeypairFromMnemonic(mnemonicData.mnemonic);
      // Экспортируем приватный ключ в Base64 для импорта
      const privateKeyBase64 = MnemonicService.exportPrivateKey(keypair.secretKey, 'base64') as string;
      await importWallet(privateKeyBase64);
      setShowMnemonic(false);
      setMnemonicData(null);
      setMnemonicWords([]);
      toast.success('Кошелек импортирован из мнемонической фразы!');
    } catch {
      toast.error('Ошибка импорта кошелька');
    }
  };

  const copyMnemonic = () => {
    if (mnemonicData) {
      navigator.clipboard.writeText(mnemonicData.mnemonic);
      toast.success('Мнемоническая фраза скопирована!');
    }
  };

  const downloadMnemonic = () => {
    if (mnemonicData) {
      const data = {
        mnemonic: mnemonicData.mnemonic,
        publicKey: mnemonicData.publicKey,
        timestamp: new Date().toISOString(),
        warning: 'НЕ ДЕЛИТЕСЬ ЭТИМ ФАЙЛОМ С НИКОМ! Храните в безопасном месте.'
      };
      
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `w3b2-wallet-backup-${Date.now()}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success('Резервная копия сохранена!');
    }
  };

  const handleGenerateWallet = () => {
    try {
      generateWallet();
      toast.success('Новый кошелек создан');
    } catch {
      toast.error('Ошибка создания кошелька');
    }
  };

  const handleImportWallet = () => {
    if (!importKey.trim()) {
      toast.error('Введите приватный ключ');
      return;
    }

    try {
      const success = importWallet(importKey);
      if (success) {
        toast.success('Кошелек импортирован');
        setImportKey('');
      } else {
        toast.error('Неверный формат приватного ключа');
      }
    } catch {
      toast.error('Ошибка импорта кошелька');
    }
  };

  const handleDisconnect = () => {
    disconnect();
    toast.success('Кошелек отключен');
  };

  const handleCopyPublicKey = () => {
    if (walletInfo.publicKey) {
      navigator.clipboard.writeText(walletInfo.publicKey.toBase58());
      toast.success('Публичный ключ скопирован');
    }
  };

  const handleCopyPrivateKey = () => {
    const privateKey = getPrivateKey();
    if (privateKey) {
      navigator.clipboard.writeText(privateKey);
      toast.success('Приватный ключ скопирован');
    }
  };

  const handleExportWallet = () => {
    const walletData = exportWallet();
    if (walletData) {
      const dataStr = JSON.stringify(walletData, null, 2);
      const dataBlob = new Blob([dataStr], { type: 'application/json' });
      const url = URL.createObjectURL(dataBlob);
      const link = document.createElement('a');
      link.href = url;
      link.download = 'wallet.json';
      link.click();
      URL.revokeObjectURL(url);
      toast.success('Кошелек экспортирован');
    }
  };

  const handleRegisterUser = async () => {
    if (!walletInfo.publicKey) return;

    try {
      setIsRegistering(true);
      const response = await registerUser(walletInfo.publicKey.toBase58());
      if (response.success) {
        toast.success('Пользователь зарегистрирован');
      } else {
        toast.error(response.error || 'Ошибка регистрации');
      }
    } catch {
      toast.error('Ошибка регистрации пользователя');
    } finally {
      setIsRegistering(false);
    }
  };

  const handleRefresh = async () => {
    try {
      await refreshBalance();
      await loadRecentTransactions();
      toast.success('Данные обновлены');
    } catch {
      toast.error('Ошибка обновления данных');
    }
  };

  return (
    <div className="space-y-6">
      {/* Заголовок */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Кошелек</h1>
          <p className="text-slate-400 mt-1">
            Управление кошельком и ключами
          </p>
        </div>
        <button
          onClick={handleRefresh}
          disabled={isLoading}
          className="btn-primary flex items-center space-x-2"
        >
          <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
          <span>Обновить</span>
        </button>
      </div>

      {/* Статус кошелька */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Статус кошелька</h3>
          <p className="card-subtitle">Информация о подключенном кошельке</p>
        </div>
        
        {walletInfo.connected && walletInfo.publicKey ? (
          <div className="space-y-4">
            {/* Баланс */}
            <div className="flex items-center justify-between p-4 bg-slate-700 rounded-lg">
              <div>
                <p className="text-sm text-slate-400">Баланс</p>
                <p className="text-2xl font-bold text-white">{balance.toFixed(4)} SOL</p>
              </div>
              <WalletIcon className="h-8 w-8 text-blue-500" />
            </div>

            {/* Публичный ключ */}
            <div className="space-y-2">
              <label className="form-label">Публичный ключ</label>
              <div className="flex items-center space-x-2">
                <input
                  type="text"
                  value={walletInfo.publicKey.toBase58()}
                  readOnly
                  className="form-input flex-1 font-mono text-sm"
                />
                <button
                  onClick={handleCopyPublicKey}
                  className="btn-outline p-2"
                  title="Копировать"
                >
                  <Copy className="h-4 w-4" />
                </button>
              </div>
            </div>

            {/* Приватный ключ */}
            <div className="space-y-2">
              <label className="form-label">Приватный ключ</label>
              <div className="flex items-center space-x-2">
                <input
                  type={showPrivateKey ? 'text' : 'password'}
                  value={getPrivateKey() || ''}
                  readOnly
                  className="form-input flex-1 font-mono text-sm"
                />
                <button
                  onClick={() => setShowPrivateKey(!showPrivateKey)}
                  className="btn-outline p-2"
                  title={showPrivateKey ? 'Скрыть' : 'Показать'}
                >
                  {showPrivateKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
                <button
                  onClick={handleCopyPrivateKey}
                  className="btn-outline p-2"
                  title="Копировать"
                >
                  <Copy className="h-4 w-4" />
                </button>
              </div>
            </div>

            {/* Действия */}
            <div className="flex space-x-3">
              <button
                onClick={handleExportWallet}
                className="btn-secondary flex items-center space-x-2"
              >
                <Download className="h-4 w-4" />
                <span>Экспорт</span>
              </button>
              <button
                onClick={handleRegisterUser}
                disabled={isRegistering}
                className="btn-primary flex items-center space-x-2"
              >
                {isRegistering ? (
                  <div className="loading-spinner"></div>
                ) : (
                  <Key className="h-4 w-4" />
                )}
                <span>{isRegistering ? 'Регистрация...' : 'Зарегистрировать'}</span>
              </button>
              <button
                onClick={handleDisconnect}
                className="btn-danger flex items-center space-x-2"
              >
                <ExternalLink className="h-4 w-4" />
                <span>Отключить</span>
              </button>
            </div>
          </div>
        ) : (
          <div className="text-center py-8">
            <WalletIcon className="h-16 w-16 text-slate-500 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Кошелек не подключен</h3>
            <p className="text-slate-400 mb-6">Создайте новый кошелек или импортируйте существующий</p>
            
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <button
                onClick={handleGenerateWallet}
                className="btn-primary flex items-center space-x-2"
              >
                <Key className="h-4 w-4" />
                <span>Создать кошелек</span>
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Импорт кошелька */}
      {!walletInfo.connected && (
        <div className="card">
          <div className="card-header">
            <h3 className="card-title">Импорт кошелька</h3>
            <p className="card-subtitle">Восстановите кошелек из приватного ключа</p>
          </div>
          
          <div className="space-y-4">
            <div className="form-group">
              <label className="form-label">Приватный ключ (Base64)</label>
              <textarea
                value={importKey}
                onChange={(e) => setImportKey(e.target.value)}
                placeholder="Введите приватный ключ в формате Base64"
                className="form-textarea"
                rows={3}
              />
            </div>
            
            <button
              onClick={handleImportWallet}
              disabled={!importKey.trim()}
              className="btn-primary flex items-center space-x-2"
            >
              <Upload className="h-4 w-4" />
              <span>Импортировать</span>
            </button>
          </div>
        </div>
      )}

      {/* Мнемонические фразы */}
      {!walletInfo.connected && (
        <div className="card">
          <div className="card-header">
            <h3 className="card-title flex items-center space-x-2">
              <Shield className="h-5 w-5" />
              <span>Мнемонические фразы</span>
            </h3>
            <p className="card-subtitle">
              Создайте или восстановите кошелек с помощью 12 слов
            </p>
          </div>
          
          <div className="space-y-6">
            {/* Генерация новой мнемонической фразы */}
            <div className="space-y-4">
              <h4 className="text-lg font-semibold text-white">Создать новый кошелек</h4>
              <p className="text-slate-400 text-sm">
                Сгенерируйте новую мнемоническую фразу для создания кошелька. 
                Сохраните эти слова в безопасном месте!
              </p>
              
              <button
                onClick={generateMnemonic}
                disabled={isGeneratingMnemonic}
                className="btn-primary flex items-center space-x-2"
              >
                {isGeneratingMnemonic ? (
                  <div className="loading-spinner"></div>
                ) : (
                  <Key className="h-4 w-4" />
                )}
                <span>
                  {isGeneratingMnemonic ? 'Генерация...' : 'Сгенерировать мнемоническую фразу'}
                </span>
              </button>
            </div>

            {/* Восстановление из мнемонической фразы */}
            <div className="space-y-4">
              <h4 className="text-lg font-semibold text-white">Восстановить кошелек</h4>
              <p className="text-slate-400 text-sm">
                Введите вашу мнемоническую фразу для восстановления кошелька
              </p>
              
              <div className="form-group">
                <label className="form-label">Мнемоническая фраза (12 слов)</label>
                <textarea
                  value={importMnemonic}
                  onChange={(e) => setImportMnemonic(e.target.value)}
                  placeholder="Введите 12 слов через пробел"
                  className="form-textarea"
                  rows={3}
                />
              </div>
              
              <button
                onClick={restoreFromMnemonic}
                disabled={!importMnemonic.trim() || isRestoringMnemonic}
                className="btn-secondary flex items-center space-x-2"
              >
                {isRestoringMnemonic ? (
                  <div className="loading-spinner"></div>
                ) : (
                  <RotateCcw className="h-4 w-4" />
                )}
                <span>
                  {isRestoringMnemonic ? 'Восстановление...' : 'Восстановить кошелек'}
                </span>
              </button>
            </div>

            {/* Отображение сгенерированной мнемонической фразы */}
            {showMnemonic && mnemonicData && (
              <div className="space-y-4 p-4 bg-slate-800 rounded-lg border border-slate-600">
                <div className="flex items-center justify-between">
                  <h4 className="text-lg font-semibold text-white">Ваша мнемоническая фраза</h4>
                  <div className="flex space-x-2">
                    <button
                      onClick={copyMnemonic}
                      className="btn-outline p-2"
                      title="Копировать"
                    >
                      <Copy className="h-4 w-4" />
                    </button>
                    <button
                      onClick={downloadMnemonic}
                      className="btn-outline p-2"
                      title="Скачать резервную копию"
                    >
                      <Download className="h-4 w-4" />
                    </button>
                  </div>
                </div>
                
                <div className="bg-slate-900 p-4 rounded-lg">
                  <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                    {mnemonicWords.map((word, index) => (
                      <div
                        key={index}
                        className="flex items-center space-x-1 bg-slate-700 px-2 py-1 rounded text-sm"
                      >
                        <span className="text-slate-400 text-xs">{index + 1}.</span>
                        <span className="text-white font-mono">{word}</span>
                      </div>
                    ))}
                  </div>
                </div>
                
                <div className="bg-yellow-900/20 border border-yellow-500/30 rounded-lg p-4">
                  <div className="flex items-start space-x-2">
                    <Shield className="h-5 w-5 text-yellow-500 mt-0.5 flex-shrink-0" />
                    <div className="text-sm">
                      <p className="text-yellow-200 font-semibold mb-1">⚠️ Важно!</p>
                      <ul className="text-yellow-300 space-y-1">
                        <li>• Сохраните эти слова в безопасном месте</li>
                        <li>• Никогда не делитесь ими с другими</li>
                        <li>• Без этих слов вы не сможете восстановить кошелек</li>
                        <li>• Запишите их на бумаге и храните отдельно от компьютера</li>
                      </ul>
                    </div>
                  </div>
                </div>
                
                <div className="flex space-x-3">
                  <button
                    onClick={useMnemonicWallet}
                    className="btn-primary flex items-center space-x-2"
                  >
                    <WalletIcon className="h-4 w-4" />
                    <span>Использовать этот кошелек</span>
                  </button>
                  <button
                    onClick={() => {
                      setShowMnemonic(false);
                      setMnemonicData(null);
                      setMnemonicWords([]);
                    }}
                    className="btn-outline"
                  >
                    Отмена
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

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
                        {tx.signature?.slice(0, 12)}...{tx.signature?.slice(-12)}
                      </p>
                      <p className="text-xs text-slate-400">
                        {tx.blockTime ? new Date(tx.blockTime * 1000).toLocaleString() : 'Неизвестно'}
                      </p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="text-sm text-green-400">Подтверждено</p>
                    <p className="text-xs text-slate-400">Slot: {tx.slot || 'N/A'}</p>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8">
                <QrCode className="h-12 w-12 text-slate-500 mx-auto mb-4" />
                <p className="text-slate-400">Нет транзакций</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default Wallet;
