import { useState, useEffect, useCallback } from 'react';
import { PublicKey } from '@solana/web3.js';
import { solanaService } from '../services/solanaService';
import { WalletInfo } from '../types/index';

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
    
    // Очищаем кошелек в сервисе
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

  // Обновление баланса при изменении кошелька
  useEffect(() => {
    if (walletInfo.connected) {
      refreshBalance();
    } else {
      setBalance(0);
    }
  }, [walletInfo.connected, refreshBalance]);

  // Проверка подключения при монтировании
  useEffect(() => {
    checkConnection();
  }, [checkConnection]);

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
  };
};
