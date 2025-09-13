import { PublicKey } from '@solana/web3.js';
import {
  DatabaseRecord,
  CrudOperation,
  FundingRequest,
  UserAccount,
  AdminAccount,
} from '../types/index';
import * as fs from 'fs/promises';
import * as path from 'path';

export class DatabaseService {
  private dataDir: string;
  private isInitialized: boolean = false;

  constructor() {
    this.dataDir = process.env.DATA_DIR || './data';
  }

  async initialize(): Promise<void> {
    try {
      // Создаем директорию для данных, если она не существует
      await fs.mkdir(this.dataDir, { recursive: true });

      // Создаем поддиректории для разных типов данных
      await fs.mkdir(path.join(this.dataDir, 'users'), { recursive: true });
      await fs.mkdir(path.join(this.dataDir, 'admins'), { recursive: true });
      await fs.mkdir(path.join(this.dataDir, 'funding_requests'), {
        recursive: true,
      });
      await fs.mkdir(path.join(this.dataDir, 'sessions'), { recursive: true });
      await fs.mkdir(path.join(this.dataDir, 'records'), { recursive: true });

      this.isInitialized = true;
      console.log('✅ База данных инициализирована');
    } catch (error) {
      console.error('❌ Ошибка инициализации базы данных:', error);
      throw error;
    }
  }

  // Работа с пользователями
  async createUser(userAccount: UserAccount): Promise<void> {
    const filePath = path.join(
      this.dataDir,
      'users',
      `${userAccount.public_key.toBase58()}.json`
    );
    const data = {
      ...userAccount,
      created_at: Date.now(),
      updated_at: Date.now(),
    };

    await fs.writeFile(filePath, JSON.stringify(data, null, 2));
    console.log(`👤 Пользователь создан: ${userAccount.public_key.toBase58()}`);
  }

  async getUser(publicKey: PublicKey): Promise<UserAccount | null> {
    try {
      const filePath = path.join(
        this.dataDir,
        'users',
        `${publicKey.toBase58()}.json`
      );
      const data = await fs.readFile(filePath, 'utf8');
      const user = JSON.parse(data);

      return {
        ...user,
        public_key: new PublicKey(user.public_key),
      };
    } catch {
      return null;
    }
  }

  async updateUser(
    publicKey: PublicKey,
    updates: Partial<UserAccount>
  ): Promise<void> {
    const user = await this.getUser(publicKey);
    if (!user) {
      throw new Error('Пользователь не найден');
    }

    const updatedUser = {
      ...user,
      ...updates,
      updated_at: Date.now(),
    };

    const filePath = path.join(
      this.dataDir,
      'users',
      `${publicKey.toBase58()}.json`
    );
    await fs.writeFile(filePath, JSON.stringify(updatedUser, null, 2));
    console.log(`👤 Пользователь обновлен: ${publicKey.toBase58()}`);
  }

  // Работа с администраторами
  async createAdmin(adminAccount: AdminAccount): Promise<void> {
    const filePath = path.join(
      this.dataDir,
      'admins',
      `${adminAccount.public_key.toBase58()}.json`
    );
    const data = {
      ...adminAccount,
      created_at: Date.now(),
      updated_at: Date.now(),
    };

    await fs.writeFile(filePath, JSON.stringify(data, null, 2));
    console.log(
      `👑 Администратор создан: ${adminAccount.public_key.toBase58()}`
    );
  }

  async getAdmin(publicKey: PublicKey): Promise<AdminAccount | null> {
    try {
      const filePath = path.join(
        this.dataDir,
        'admins',
        `${publicKey.toBase58()}.json`
      );
      const data = await fs.readFile(filePath, 'utf8');
      const admin = JSON.parse(data);

      return {
        ...admin,
        public_key: new PublicKey(admin.public_key),
      };
    } catch {
      return null;
    }
  }

  // Работа с запросами на финансирование
  async createFundingRequest(fundingRequest: FundingRequest): Promise<string> {
    const requestId = `${fundingRequest.user_wallet.toBase58()}_${Date.now()}`;
    const filePath = path.join(
      this.dataDir,
      'funding_requests',
      `${requestId}.json`
    );

    const data = {
      id: requestId,
      ...fundingRequest,
      created_at: Date.now(),
      updated_at: Date.now(),
    };

    await fs.writeFile(filePath, JSON.stringify(data, null, 2));
    console.log(`💰 Запрос на финансирование создан: ${requestId}`);

    return requestId;
  }

  async getFundingRequest(requestId: string): Promise<FundingRequest | null> {
    try {
      const filePath = path.join(
        this.dataDir,
        'funding_requests',
        `${requestId}.json`
      );
      const data = await fs.readFile(filePath, 'utf8');
      const request = JSON.parse(data);

      return {
        ...request,
        user_wallet: new PublicKey(request.user_wallet),
        target_admin: new PublicKey(request.target_admin),
      };
    } catch {
      return null;
    }
  }

  async updateFundingRequest(
    requestId: string,
    updates: Partial<FundingRequest>
  ): Promise<void> {
    const request = await this.getFundingRequest(requestId);
    if (!request) {
      throw new Error('Запрос на финансирование не найден');
    }

    const updatedRequest = {
      ...request,
      ...updates,
      updated_at: Date.now(),
    };

    const filePath = path.join(
      this.dataDir,
      'funding_requests',
      `${requestId}.json`
    );
    await fs.writeFile(filePath, JSON.stringify(updatedRequest, null, 2));
    console.log(`💰 Запрос на финансирование обновлен: ${requestId}`);
  }

  async getAllFundingRequests(): Promise<FundingRequest[]> {
    try {
      const dirPath = path.join(this.dataDir, 'funding_requests');
      const files = await fs.readdir(dirPath);

      const requests: FundingRequest[] = [];

      for (const file of files) {
        if (file.endsWith('.json')) {
          const filePath = path.join(dirPath, file);
          const data = await fs.readFile(filePath, 'utf8');
          const request = JSON.parse(data);

          requests.push({
            ...request,
            user_wallet: new PublicKey(request.user_wallet),
            target_admin: new PublicKey(request.target_admin),
          });
        }
      }

      return requests;
    } catch (error) {
      console.error('❌ Ошибка получения запросов на финансирование:', error);
      return [];
    }
  }

  // CRUD операции с записями
  async createRecord(owner: PublicKey, data: any): Promise<string> {
    const recordId = `record_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const record: DatabaseRecord = {
      id: recordId,
      data,
      created_at: Date.now(),
      updated_at: Date.now(),
      owner,
    };

    const filePath = path.join(this.dataDir, 'records', `${recordId}.json`);
    await fs.writeFile(filePath, JSON.stringify(record, null, 2));
    console.log(`📝 Запись создана: ${recordId}`);

    return recordId;
  }

  async getRecord(recordId: string): Promise<DatabaseRecord | null> {
    try {
      const filePath = path.join(this.dataDir, 'records', `${recordId}.json`);
      const data = await fs.readFile(filePath, 'utf8');
      const record = JSON.parse(data);

      return {
        ...record,
        owner: new PublicKey(record.owner),
      };
    } catch {
      return null;
    }
  }

  async updateRecord(
    recordId: string,
    updates: any,
    owner: PublicKey
  ): Promise<void> {
    const record = await this.getRecord(recordId);
    if (!record) {
      throw new Error('Запись не найдена');
    }

    if (!record.owner.equals(owner)) {
      throw new Error('Недостаточно прав для обновления записи');
    }

    const updatedRecord = {
      ...record,
      data: { ...record.data, ...updates },
      updated_at: Date.now(),
    };

    const filePath = path.join(this.dataDir, 'records', `${recordId}.json`);
    await fs.writeFile(filePath, JSON.stringify(updatedRecord, null, 2));
    console.log(`📝 Запись обновлена: ${recordId}`);
  }

  async deleteRecord(recordId: string, owner: PublicKey): Promise<void> {
    const record = await this.getRecord(recordId);
    if (!record) {
      throw new Error('Запись не найдена');
    }

    if (!record.owner.equals(owner)) {
      throw new Error('Недостаточно прав для удаления записи');
    }

    const filePath = path.join(this.dataDir, 'records', `${recordId}.json`);
    await fs.unlink(filePath);
    console.log(`🗑️ Запись удалена: ${recordId}`);
  }

  async getRecordsByOwner(owner: PublicKey): Promise<DatabaseRecord[]> {
    try {
      const dirPath = path.join(this.dataDir, 'records');
      const files = await fs.readdir(dirPath);

      const records: DatabaseRecord[] = [];

      for (const file of files) {
        if (file.endsWith('.json')) {
          const filePath = path.join(dirPath, file);
          const data = await fs.readFile(filePath, 'utf8');
          const record = JSON.parse(data);

          if (new PublicKey(record.owner).equals(owner)) {
            records.push({
              ...record,
              owner: new PublicKey(record.owner),
            });
          }
        }
      }

      return records;
    } catch (error) {
      console.error('❌ Ошибка получения записей:', error);
      return [];
    }
  }

  // Обработка CRUD операций
  async handleCrudOperation(
    operation: CrudOperation,
    owner: PublicKey
  ): Promise<any> {
    switch (operation.type) {
      case 'create':
        return await this.createRecord(owner, operation.data);

      case 'read':
        if (operation.id) {
          return await this.getRecord(operation.id);
        } else {
          return await this.getRecordsByOwner(owner);
        }

      case 'update':
        if (!operation.id) {
          throw new Error('ID записи обязателен для обновления');
        }
        await this.updateRecord(operation.id, operation.data, owner);
        return { success: true };

      case 'delete':
        if (!operation.id) {
          throw new Error('ID записи обязателен для удаления');
        }
        await this.deleteRecord(operation.id, owner);
        return { success: true };

      default:
        throw new Error('Неизвестный тип операции');
    }
  }

  // Статистика
  async getStats(): Promise<{
    users: number;
    admins: number;
    fundingRequests: number;
    records: number;
  }> {
    try {
      const usersDir = path.join(this.dataDir, 'users');
      const adminsDir = path.join(this.dataDir, 'admins');
      const fundingRequestsDir = path.join(this.dataDir, 'funding_requests');
      const recordsDir = path.join(this.dataDir, 'records');

      const [usersFiles, adminsFiles, fundingRequestsFiles, recordsFiles] =
        await Promise.all([
          fs.readdir(usersDir).catch(() => []),
          fs.readdir(adminsDir).catch(() => []),
          fs.readdir(fundingRequestsDir).catch(() => []),
          fs.readdir(recordsDir).catch(() => []),
        ]);

      return {
        users: usersFiles.filter(f => f.endsWith('.json')).length,
        admins: adminsFiles.filter(f => f.endsWith('.json')).length,
        fundingRequests: fundingRequestsFiles.filter(f => f.endsWith('.json'))
          .length,
        records: recordsFiles.filter(f => f.endsWith('.json')).length,
      };
    } catch (error) {
      console.error('❌ Ошибка получения статистики:', error);
      return { users: 0, admins: 0, fundingRequests: 0, records: 0 };
    }
  }

  // Очистка старых данных
  async cleanupOldData(
    maxAge: number = 30 * 24 * 60 * 60 * 1000
  ): Promise<void> {
    const now = Date.now();
    const cutoff = now - maxAge;

    try {
      const dirs = ['users', 'admins', 'funding_requests', 'records'];

      for (const dir of dirs) {
        const dirPath = path.join(this.dataDir, dir);
        const files = await fs.readdir(dirPath);

        for (const file of files) {
          if (file.endsWith('.json')) {
            const filePath = path.join(dirPath, file);
            const stats = await fs.stat(filePath);

            if (stats.mtime.getTime() < cutoff) {
              await fs.unlink(filePath);
              console.log(`🧹 Удален старый файл: ${file}`);
            }
          }
        }
      }
    } catch (error) {
      console.error('❌ Ошибка очистки старых данных:', error);
    }
  }
}
