import { useState, useCallback } from 'react';
import { apiService } from '../services/apiService';
import type{ 
  CrudOperation
} from '../types/index';

export const useApi = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Обработка ошибок API
  const handleApiError = useCallback((error: any) => {
    const errorMessage = error.response?.data?.error || error.message || 'Неизвестная ошибка';
    setError(errorMessage);
    console.error('API Error:', error);
  }, []);

  // Очистка ошибки
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Регистрация пользователя
  const registerUser = useCallback(async (publicKey: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.registerUser(publicKey);
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Получение информации о пользователе
  const getUser = useCallback(async (publicKey: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.getUser(publicKey);
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Запрос на финансирование
  const requestFunding = useCallback(async (
    userWallet: string, 
    amount: number, 
    targetAdmin: string,
    userPrivateKey: string
  ) => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.requestFunding(userWallet, amount, targetAdmin, userPrivateKey);
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Одобрение финансирования
  const approveFunding = useCallback(async (requestId: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.approveFunding(requestId);
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Получение всех запросов на финансирование
  const getFundingRequests = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.getFundingRequests();
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // CRUD операции
  const performCrudOperation = useCallback(async (operation: CrudOperation, owner: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.performCrudOperation(operation, owner);
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Создание записи
  const createRecord = useCallback(async (owner: string, data: any) => {
    try {
      setIsLoading(true);
      setError(null);
      const recordId = await apiService.createRecord(owner, data);
      return recordId;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Получение записи
  const getRecord = useCallback(async (recordId: string, owner: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const record = await apiService.getRecord(recordId, owner);
      return record;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Получение всех записей пользователя
  const getUserRecords = useCallback(async (owner: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const records = await apiService.getUserRecords(owner);
      return records;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Обновление записи
  const updateRecord = useCallback(async (recordId: string, data: any, owner: string) => {
    try {
      setIsLoading(true);
      setError(null);
      await apiService.updateRecord(recordId, data, owner);
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Удаление записи
  const deleteRecord = useCallback(async (recordId: string, owner: string) => {
    try {
      setIsLoading(true);
      setError(null);
      await apiService.deleteRecord(recordId, owner);
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Создание сессии
  const createSession = useCallback(async (clientPublicKey: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.createSession(clientPublicKey);
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Закрытие сессии
  const closeSession = useCallback(async (sessionId: number) => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.closeSession(sessionId);
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Получение статистики
  const getStats = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await apiService.getStats();
      return response;
    } catch (error) {
      handleApiError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [handleApiError]);

  // Проверка состояния API
  const healthCheck = useCallback(async () => {
    try {
      const isHealthy = await apiService.healthCheck();
      return isHealthy;
    } catch (error) {
      console.error('Health check failed:', error);
      return false;
    }
  }, []);

  return {
    isLoading,
    error,
    clearError,
    registerUser,
    getUser,
    requestFunding,
    approveFunding,
    getFundingRequests,
    performCrudOperation,
    createRecord,
    getRecord,
    getUserRecords,
    updateRecord,
    deleteRecord,
    createSession,
    closeSession,
    getStats,
    healthCheck,
  };
};
