import React, { createContext, useContext, type ReactNode } from 'react';
import { useWallet } from '../hooks/useWallet';
import { type WalletInfo } from '../types/index';

interface WalletContextType {
  walletInfo: WalletInfo;
  balance: number;
  isLoading: boolean;
  initializeWallet: (privateKey?: string) => boolean;
  generateWallet: () => any;
  importWallet: (privateKey: string) => boolean;
  disconnect: () => void;
  refreshBalance: () => Promise<void>;
  exportWallet: () => { publicKey: string; privateKey: string } | null;
  getPrivateKey: () => string | null;
  checkConnection: () => Promise<boolean>;
  getRecentTransactions: (limit?: number) => Promise<any[]>;
}

const WalletContext = createContext<WalletContextType | undefined>(undefined);

interface WalletProviderProps {
  children: ReactNode;
}

export const WalletProvider: React.FC<WalletProviderProps> = ({ children }) => {
  const wallet = useWallet();

  return (
    <WalletContext.Provider value={wallet}>
      {children}
    </WalletContext.Provider>
  );
};

export const useWalletContext = (): WalletContextType => {
  const context = useContext(WalletContext);
  if (context === undefined) {
    throw new Error('useWalletContext must be used within a WalletProvider');
  }
  return context;
};
