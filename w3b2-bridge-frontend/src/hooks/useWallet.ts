import { useState, useEffect, useCallback } from 'react';
import { solanaService } from '../services/solanaService';
import type { WalletInfo } from '../types/index';

export const useWallet = () => {
  const [walletInfo, setWalletInfo] = useState<WalletInfo>({
    publicKey: null,
    connected: false,
    connecting: false,
    disconnecting: false,
  });

  const [balance, setBalance] = useState<number>(0);
  const [isLoading, setIsLoading] = useState(false);

  // Инициализация кошелька
  const initializeWallet = useCallback((privateKey?: string) => {
    try {
      const wallet = solanaService.initializeWallet(privateKey);
      setWalletInfo({
        publicKey: wallet.publicKey,
        connected: true,
        connecting: false,
        disconnecting: false,
      });
      return true;
    } catch (error) {
      console.error('Ошибка инициализации кошелька:', error);
      return false;
    }
  }, []);

  // Генерация нового кошелька
  const generateWallet = useCallback(() => {
    try {
      const wallet = solanaService.generateNewWallet();
      setWalletInfo({
        publicKey: wallet.publicKey,
        connected: true,
        connecting: false,
        disconnecting: false,
      });
      return wallet;
    } catch (error) {
      console.error('Ошибка генерации кошелька:', error);
      return null;
    }
  }, []);

  // Импорт кошелька
  const importWallet = useCallback((privateKey: string) => {
    const success = solanaService.importWallet(privateKey);
    if (success) {
      const publicKey = solanaService.getPublicKey();
      setWalletInfo({
        publicKey,
        connected: true,
        connecting: false,
        disconnecting: false,
      });
    }
    return success;
  }, []);

  // Отключение кошелька
  const disconnect = useCallback(() => {
    setWalletInfo({
      publicKey: null,
      connected: false,
      connecting: false,
      disconnecting: true,
    });
    
    // Очищаем кошелек в сервисе и localStorage
    solanaService.clearWalletFromStorage();
    solanaService.initializeWallet();
    
    setTimeout(() => {
      setWalletInfo({
        publicKey: null,
        connected: false,
        connecting: false,
        disconnecting: false,
      });
    }, 1000);
  }, []);

  // Получение баланса
  const refreshBalance = useCallback(async () => {
    if (!walletInfo.connected) return;

    try {
      setIsLoading(true);
      const newBalance = await solanaService.getBalance();
      setBalance(newBalance);
    } catch (error) {
      console.error('Ошибка получения баланса:', error);
    } finally {
      setIsLoading(false);
    }
  }, [walletInfo.connected]);

  // Экспорт кошелька
  const exportWallet = useCallback(() => {
    return solanaService.exportWallet();
  }, []);

  // Получение приватного ключа
  const getPrivateKey = useCallback(() => {
    return solanaService.getPrivateKey();
  }, []);

  // Проверка подключения к сети
  const checkConnection = useCallback(async () => {
    try {
      const isConnected = await solanaService.isConnected();
      if (!isConnected) {
        console.warn('Нет подключения к Solana сети');
      }
      return isConnected;
    } catch (error) {
      console.error('Ошибка проверки подключения:', error);
      return false;
    }
  }, []);

  // Получение последних транзакций
  const getRecentTransactions = useCallback(async (limit: number = 10) => {
    if (!walletInfo.connected) return [];

    try {
      return await solanaService.getRecentTransactions(limit);
    } catch (error) {
      console.error('Ошибка получения транзакций:', error);
      return [];
    }
  }, [walletInfo.connected]);

  // Запрос airdrop для тестовой сети
  const requestAirdrop = useCallback(async (lamports?: number) => {
    if (!walletInfo.connected) return null;

    try {
      setIsLoading(true);
      const signature = await solanaService.requestAirdrop(lamports);
      // Обновляем баланс после получения airdrop
      await refreshBalance();
      return signature;
    } catch (error) {
      console.error('Ошибка получения airdrop:', error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [walletInfo.connected, refreshBalance]);

  // Обновление баланса при изменении кошелька
  useEffect(() => {
    if (walletInfo.connected) {
      refreshBalance();
    } else {
      setBalance(0);
    }
  }, [walletInfo.connected, refreshBalance]);

  // Инициализация кошелька при монтировании
  useEffect(() => {
    const initializeOnMount = async () => {
      // Сначала проверяем подключение к сети
      const isConnected = await checkConnection();
      if (isConnected) {
        // Если есть подключение, инициализируем кошелек
        const success = initializeWallet();
        if (success) {
          console.log('✅ Кошелек инициализирован при загрузке');
        }
      }
    };
    
    initializeOnMount();
  }, [checkConnection, initializeWallet]);

  return {
    walletInfo,
    balance,
    isLoading,
    initializeWallet,
    generateWallet,
    importWallet,
    disconnect,
    refreshBalance,
    exportWallet,
    getPrivateKey,
    checkConnection,
    getRecentTransactions,
    requestAirdrop,
  };
};
