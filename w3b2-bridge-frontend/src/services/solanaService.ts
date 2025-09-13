import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  SystemProgram,
  LAMPORTS_PER_SOL,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { CommandId, CommandMode, Destination } from '../types/index.js';

export class SolanaService {
  private connection: Connection;
  private programId: PublicKey;
  private wallet: Keypair | null = null;

  constructor() {
    this.connection = new Connection(
      process.env.REACT_APP_SOLANA_RPC_URL || 'https://api.devnet.solana.com',
      'confirmed'
    );
    this.programId = new PublicKey(
      process.env.REACT_APP_PROGRAM_ID || 'W3B2Bridge111111111111111111111111111111111'
    );
  }

  // Инициализация кошелька
  initializeWallet(privateKey?: string): Keypair {
    if (privateKey) {
      try {
        // Декодируем приватный ключ из base58
        const secretKey = Buffer.from(privateKey, 'base64');
        this.wallet = Keypair.fromSecretKey(secretKey);
      } catch (error) {
        console.error('Ошибка загрузки приватного ключа:', error);
        this.wallet = Keypair.generate();
      }
    } else {
      this.wallet = Keypair.generate();
    }

    console.log('🔑 Кошелек инициализирован:', this.wallet.publicKey.toBase58());
    return this.wallet;
  }

  // Получение публичного ключа
  getPublicKey(): PublicKey | null {
    return this.wallet?.publicKey || null;
  }

  // Получение приватного ключа (для экспорта)
  getPrivateKey(): string | null {
    if (!this.wallet) return null;
    return Buffer.from(this.wallet.secretKey).toString('base64');
  }

  // Получение баланса
  async getBalance(): Promise<number> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');
    
    const balance = await this.connection.getBalance(this.wallet.publicKey);
    return balance / LAMPORTS_PER_SOL;
  }

  // Запрос на финансирование
  async requestFunding(amount: number, targetAdmin: string): Promise<string> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');

    const transaction = new Transaction();
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: new PublicKey(targetAdmin), isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data: Buffer.from([
        CommandId.REQUEST_CONNECTION, // Используем команду запроса соединения
        ...Buffer.alloc(8).fill(amount), // amount
      ]),
    });

    transaction.add(instruction);
    
    const signature = await sendAndConfirmTransaction(
      this.connection,
      transaction,
      [this.wallet]
    );
    
    console.log('💰 Запрос на финансирование отправлен:', signature);
    return signature;
  }

  // Публикация публичного ключа
  async publishPublicKey(): Promise<string> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');

    const transaction = new Transaction();
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data: Buffer.from([
        CommandId.PUBLISH_PUBKEY,
        ...this.wallet.publicKey.toBytes(),
      ]),
    });

    transaction.add(instruction);
    
    const signature = await sendAndConfirmTransaction(
      this.connection,
      transaction,
      [this.wallet]
    );
    
    console.log('🔑 Публичный ключ опубликован:', signature);
    return signature;
  }

  // Отправка команды
  async dispatchCommand(
    commandId: CommandId,
    mode: CommandMode,
    payload: Uint8Array,
    targetAdmin: string
  ): Promise<string> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');

    const transaction = new Transaction();
    
    const data = Buffer.concat([
      Buffer.from([commandId, mode]),
      Buffer.from(payload)
    ]);

    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: new PublicKey(targetAdmin), isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data,
    });

    transaction.add(instruction);
    
    const signature = await sendAndConfirmTransaction(
      this.connection,
      transaction,
      [this.wallet]
    );
    
    console.log(`📤 Команда ${commandId} отправлена:`, signature);
    return signature;
  }

  // CRUD операции
  async createRecord(data: any, targetAdmin: string): Promise<string> {
    const payload = this.serializeData(data);
    return await this.dispatchCommand(
      CommandId.CRUD_CREATE,
      CommandMode.RequestResponse,
      payload,
      targetAdmin
    );
  }

  async readRecord(recordId: string, targetAdmin: string): Promise<string> {
    const payload = this.serializeData({ id: recordId });
    return await this.dispatchCommand(
      CommandId.CRUD_READ,
      CommandMode.RequestResponse,
      payload,
      targetAdmin
    );
  }

  async updateRecord(recordId: string, data: any, targetAdmin: string): Promise<string> {
    const payload = this.serializeData({ id: recordId, data });
    return await this.dispatchCommand(
      CommandId.CRUD_UPDATE,
      CommandMode.RequestResponse,
      payload,
      targetAdmin
    );
  }

  async deleteRecord(recordId: string, targetAdmin: string): Promise<string> {
    const payload = this.serializeData({ id: recordId });
    return await this.dispatchCommand(
      CommandId.CRUD_DELETE,
      CommandMode.RequestResponse,
      payload,
      targetAdmin
    );
  }

  // Управление сессиями
  async startSession(destination: Destination, targetAdmin: string): Promise<string> {
    const payload = this.serializeData({ destination });
    return await this.dispatchCommand(
      CommandId.START_SESSION,
      CommandMode.RequestResponse,
      payload,
      targetAdmin
    );
  }

  async endSession(targetAdmin: string): Promise<string> {
    return await this.dispatchCommand(
      CommandId.END_SESSION,
      CommandMode.OneWay,
      new Uint8Array(),
      targetAdmin
    );
  }

  // Вспомогательные методы
  private serializeData(data: any): Uint8Array {
    // Простая сериализация JSON (в реальном проекте используйте Borsh)
    const jsonString = JSON.stringify(data);
    return new TextEncoder().encode(jsonString);
  }

  private deserializeData(data: Uint8Array): any {
    const jsonString = new TextDecoder().decode(data);
    return JSON.parse(jsonString);
  }

  // Получение информации о программе
  getProgramId(): PublicKey {
    return this.programId;
  }

  getConnection(): Connection {
    return this.connection;
  }

  // Проверка подключения к сети
  async isConnected(): Promise<boolean> {
    try {
      await this.connection.getVersion();
      return true;
    } catch (error) {
      return false;
    }
  }

  // Получение последних транзакций
  async getRecentTransactions(limit: number = 10): Promise<any[]> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');

    const signatures = await this.connection.getSignaturesForAddress(
      this.wallet.publicKey,
      { limit }
    );

    return signatures;
  }

  // Получение информации о транзакции
  async getTransactionInfo(signature: string): Promise<any> {
    const transaction = await this.connection.getTransaction(signature, {
      commitment: 'confirmed',
      maxSupportedTransactionVersion: 0,
    });

    return transaction;
  }

  // Генерация нового кошелька
  generateNewWallet(): Keypair {
    this.wallet = Keypair.generate();
    console.log('🔑 Новый кошелек сгенерирован:', this.wallet.publicKey.toBase58());
    return this.wallet;
  }

  // Экспорт кошелька
  exportWallet(): { publicKey: string; privateKey: string } | null {
    if (!this.wallet) return null;

    return {
      publicKey: this.wallet.publicKey.toBase58(),
      privateKey: this.getPrivateKey()!,
    };
  }

  // Импорт кошелька
  importWallet(privateKey: string): boolean {
    try {
      const secretKey = Buffer.from(privateKey, 'base64');
      this.wallet = Keypair.fromSecretKey(secretKey);
      console.log('🔑 Кошелек импортирован:', this.wallet.publicKey.toBase58());
      return true;
    } catch (error) {
      console.error('Ошибка импорта кошелька:', error);
      return false;
    }
  }
}

// Экспортируем singleton instance
export const solanaService = new SolanaService();
export default solanaService;
