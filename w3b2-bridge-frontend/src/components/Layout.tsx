import React, { useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { 
  Home, 
  Wallet, 
  CreditCard, 
  Database, 
  Settings, 
  Menu, 
  X,
  Activity,
  Shield
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';

interface LayoutProps {
  children: React.ReactNode;
}

const Layout: React.FC<LayoutProps> = ({ children }) => {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const { walletInfo, balance } = useWalletContext();
  const location = useLocation();

  const navigation = [
    { name: 'Панель управления', href: '/', icon: Home },
    { name: 'Кошелек', href: '/wallet', icon: Wallet },
    { name: 'Финансирование', href: '/funding', icon: CreditCard },
    { name: 'Записи', href: '/records', icon: Database },
    { name: 'Настройки', href: '/settings', icon: Settings },
  ];

  const isActive = (href: string) => {
    return location.pathname === href;
  };

  return (
    <div className="min-h-screen bg-slate-900">
      {/* Мобильное меню */}
      <div className="lg:hidden">
        <div className="flex items-center justify-between p-4 bg-slate-800 border-b border-slate-700">
          <div className="flex items-center space-x-2">
            <Shield className="h-8 w-8 text-primary-500" />
            <span className="text-xl font-bold text-white">W3B2</span>
          </div>
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="text-slate-400 hover:text-white"
          >
            {sidebarOpen ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
          </button>
        </div>
      </div>

      <div className="flex">
        {/* Боковая панель */}
        <div className={`fixed inset-y-0 left-0 z-50 w-64 bg-slate-800 transform transition-transform duration-300 ease-in-out lg:translate-x-0 lg:static lg:inset-0 ${
          sidebarOpen ? 'translate-x-0' : '-translate-x-full'
        }`}>
          <div className="flex flex-col h-full">
            {/* Логотип */}
            <div className="flex items-center space-x-2 p-6 border-b border-slate-700">
              <Shield className="h-8 w-8 text-primary-500" />
              <span className="text-xl font-bold text-white">W3B2 Bridge</span>
            </div>

            {/* Статус кошелька */}
            <div className="p-4 border-b border-slate-700">
              <div className="flex items-center space-x-2 mb-2">
                <Activity className={`h-4 w-4 ${walletInfo.connected ? 'text-green-400' : 'text-red-400'}`} />
                <span className="text-sm text-slate-300">
                  {walletInfo.connected ? 'Подключено' : 'Отключено'}
                </span>
              </div>
              {walletInfo.connected && walletInfo.publicKey && (
                <div className="space-y-1">
                  <div className="text-xs text-slate-400">Публичный ключ:</div>
                  <div className="text-xs font-mono text-slate-300 break-all">
                    {walletInfo.publicKey.toBase58().slice(0, 8)}...{walletInfo.publicKey.toBase58().slice(-8)}
                  </div>
                  <div className="text-xs text-slate-400">
                    Баланс: {balance.toFixed(4)} SOL
                  </div>
                </div>
              )}
            </div>

            {/* Навигация */}
            <nav className="flex-1 p-4 space-y-2">
              {navigation.map((item) => {
                const Icon = item.icon;
                return (
                  <Link
                    key={item.name}
                    to={item.href}
                    onClick={() => setSidebarOpen(false)}
                    className={`flex items-center space-x-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      isActive(item.href)
                        ? 'bg-primary-600 text-white'
                        : 'text-slate-300 hover:bg-slate-700 hover:text-white'
                    }`}
                  >
                    <Icon className="h-5 w-5" />
                    <span>{item.name}</span>
                  </Link>
                );
              })}
            </nav>

            {/* Информация о версии */}
            <div className="p-4 border-t border-slate-700">
              <div className="text-xs text-slate-500">
                W3B2 Bridge v1.0.0
              </div>
              <div className="text-xs text-slate-500">
                Solana Devnet
              </div>
            </div>
          </div>
        </div>

        {/* Основной контент */}
        <div className="flex-1 flex flex-col min-h-screen">
          {/* Верхняя панель для мобильных устройств */}
          <div className="lg:hidden bg-slate-800 border-b border-slate-700 p-4">
            <div className="flex items-center justify-between">
              <h1 className="text-lg font-semibold text-white">
                {navigation.find(item => item.href === location.pathname)?.name || 'W3B2 Bridge'}
              </h1>
              {walletInfo.connected && (
                <div className="flex items-center space-x-2">
                  <div className="w-2 h-2 bg-green-400 rounded-full"></div>
                  <span className="text-sm text-slate-300">Подключено</span>
                </div>
              )}
            </div>
          </div>

          {/* Контент страницы */}
          <main className="flex-1 p-6">
            {children}
          </main>
        </div>
      </div>

      {/* Overlay для мобильного меню */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 z-40 bg-black bg-opacity-50 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}
    </div>
  );
};

export default Layout;
