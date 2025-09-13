import React, { createContext, useContext, type ReactNode } from 'react';
import { useApi } from '../hooks/useApi';

interface ApiContextType {
  isLoading: boolean;
  error: string | null;
  clearError: () => void;
  registerUser: (publicKey: string) => Promise<any>;
  getUser: (publicKey: string) => Promise<any>;
  requestFunding: (userWallet: string, amount: number, targetAdmin: string, userPrivateKey: string) => Promise<any>;
  approveFunding: (requestId: string) => Promise<any>;
  getFundingRequests: () => Promise<any>;
  performCrudOperation: (operation: any, owner: string) => Promise<any>;
  createRecord: (owner: string, data: any) => Promise<string>;
  getRecord: (recordId: string, owner: string) => Promise<any>;
  getUserRecords: (owner: string) => Promise<any[]>;
  updateRecord: (recordId: string, data: any, owner: string) => Promise<void>;
  deleteRecord: (recordId: string, owner: string) => Promise<void>;
  createSession: (clientPublicKey: string) => Promise<any>;
  closeSession: (sessionId: number) => Promise<any>;
  getStats: () => Promise<any>;
  healthCheck: () => Promise<boolean>;
}

const ApiContext = createContext<ApiContextType | undefined>(undefined);

interface ApiProviderProps {
  children: ReactNode;
}

export const ApiProvider: React.FC<ApiProviderProps> = ({ children }) => {
  const api = useApi();

  return (
    <ApiContext.Provider value={api}>
      {children}
    </ApiContext.Provider>
  );
};

export const useApiContext = (): ApiContextType => {
  const context = useContext(ApiContext);
  if (context === undefined) {
    throw new Error('useApiContext must be used within an ApiProvider');
  }
  return context;
};
