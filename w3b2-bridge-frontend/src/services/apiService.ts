import axios, { type AxiosInstance } from 'axios';
import type { 
  ApiResponse, 
  UserAccount, 
  FundingRequest, 
  CrudOperation, 
  SessionData, 
  AppStats,
  DatabaseRecord,
  CommandConfig,
  CommandId,
  CommandMode
} from '../types/index';

class ApiService {
  private api: AxiosInstance;
  private baseURL: string;

  constructor() {
    this.baseURL = process.env.REACT_APP_API_URL || 'http://localhost:3001/api';
    this.api = axios.create({
      baseURL: this.baseURL,
      timeout: 10000,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Interceptor для обработки ошибок
    this.api.interceptors.response.use(
      (response) => response,
      (error) => {
        console.error('API Error:', error);
        return Promise.reject(error);
      }
    );
  }

  // Регистрация пользователя
  async registerUser(publicKey: string): Promise<ApiResponse<{ publicKey: string; serverPublicKey: string }>> {
    const response = await this.api.post('/register-user', { publicKey });
    return response.data;
  }

  // Получение информации о пользователе
  async getUser(publicKey: string): Promise<ApiResponse<UserAccount>> {
    const response = await this.api.get(`/user/${publicKey}`);
    return response.data;
  }

  // Запрос на финансирование
  async requestFunding(
    userWallet: string, 
    amount: number, 
    targetAdmin: string
  ): Promise<ApiResponse<{ requestId: string; signature: string; message: string }>> {
    const response = await this.api.post('/request-funding', {
      userWallet,
      amount,
      targetAdmin,
    });
    return response.data;
  }

  // Одобрение финансирования
  async approveFunding(requestId: string): Promise<ApiResponse<{ requestId: string; status: string; signature: string; message: string }>> {
    const response = await this.api.post('/approve-funding', { requestId });
    return response.data;
  }

  // Получение всех запросов на финансирование
  async getFundingRequests(): Promise<ApiResponse<FundingRequest[]>> {
    const response = await this.api.get('/funding-requests');
    return response.data;
  }

  // Отправка команды в blockchain
  async dispatchCommand(
    commandId: CommandId,
    mode: CommandMode,
    config: CommandConfig,
    targetAdmin: string
  ): Promise<ApiResponse<{ signature: string; commandId: number; mode: number; message: string }>> {
    const response = await this.api.post('/dispatch-command', {
      commandId,
      mode,
      config,
      targetAdmin,
    });
    return response.data;
  }

  // CRUD операции
  async performCrudOperation(operation: CrudOperation, owner: string): Promise<ApiResponse<unknown>> {
    const response = await this.api.post('/crud', { operation, owner });
    return response.data;
  }

  // Создание сессии
  async createSession(clientPublicKey: string): Promise<ApiResponse<SessionData>> {
    const response = await this.api.post('/session/create', { clientPublicKey });
    return response.data;
  }

  // Закрытие сессии
  async closeSession(sessionId: number): Promise<ApiResponse<{ sessionId: number; closed: boolean }>> {
    const response = await this.api.post('/session/close', { sessionId });
    return response.data;
  }

  // Получение статистики
  async getStats(): Promise<ApiResponse<AppStats>> {
    const response = await this.api.get('/stats');
    return response.data;
  }

  // Создание записи
  async createRecord(owner: string, data: unknown): Promise<string> {
    const operation: CrudOperation = {
      type: 'create',
      data,
    };
    
    const response = await this.performCrudOperation(operation, owner);
    return response.data as string;
  }

  // Получение записи
  async getRecord(recordId: string, owner: string): Promise<DatabaseRecord | null> {
    const operation: CrudOperation = {
      type: 'read',
      id: recordId,
    };
    
    const response = await this.performCrudOperation(operation, owner);
    return response.data as DatabaseRecord | null;
  }

  // Получение всех записей пользователя
  async getUserRecords(owner: string): Promise<DatabaseRecord[]> {
    const operation: CrudOperation = {
      type: 'read',
    };
    
    const response = await this.performCrudOperation(operation, owner);
    return response.data || [];
  }

  // Обновление записи
  async updateRecord(recordId: string, data: unknown, owner: string): Promise<void> {
    const operation: CrudOperation = {
      type: 'update',
      id: recordId,
      data,
    };
    
    await this.performCrudOperation(operation, owner);
  }

  // Удаление записи
  async deleteRecord(recordId: string, owner: string): Promise<void> {
    const operation: CrudOperation = {
      type: 'delete',
      id: recordId,
    };
    
    await this.performCrudOperation(operation, owner);
  }

  // Проверка состояния API
  async healthCheck(): Promise<boolean> {
    try {
      const response = await this.api.get('/health');
      return response.status === 200;
    } catch {
      return false;
    }
  }

  // Получение базового URL
  getBaseURL(): string {
    return this.baseURL;
  }

  // Установка токена авторизации (если понадобится в будущем)
  setAuthToken(token: string): void {
    this.api.defaults.headers.common['Authorization'] = `Bearer ${token}`;
  }

  // Удаление токена авторизации
  removeAuthToken(): void {
    delete this.api.defaults.headers.common['Authorization'];
  }
}

// Экспортируем singleton instance
export const apiService = new ApiService();
export default apiService;
