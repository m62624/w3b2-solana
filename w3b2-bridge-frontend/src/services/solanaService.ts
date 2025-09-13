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
import { Buffer } from 'buffer';
import { CommandId, CommandMode, type Destination } from '../types/index';

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
      process.env.REACT_APP_PROGRAM_ID || '3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr'
    );
  }

  // Инициализация кошелька
  initializeWallet(privateKey?: string): Keypair {
    if (privateKey) {
      try {
        // Декодируем приватный ключ из base64
        const secretKey = Buffer.from(privateKey, 'base64');
        this.wallet = Keypair.fromSecretKey(secretKey);
        this.saveWalletToStorage();
      } catch (error) {
        console.error('Ошибка загрузки приватного ключа:', error);
        this.wallet = Keypair.generate();
        this.saveWalletToStorage();
      }
    } else {
      // Проверяем localStorage для сохраненного кошелька
      const savedWallet = this.loadWalletFromStorage();
      if (savedWallet) {
        this.wallet = savedWallet;
        console.log('📂 Кошелек загружен из localStorage:', this.wallet.publicKey.toBase58());
      } else {
        this.wallet = Keypair.generate();
        this.saveWalletToStorage();
        console.log('🔑 Новый кошелек сгенерирован:', this.wallet.publicKey.toBase58());
      }
    }

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

  // Запрос на финансирование согласно W3B2 Bridge Protocol
  async requestFunding(amount: number, targetAdmin: string): Promise<string> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');

    const transaction = new Transaction();
    
    // Находим PDA для funding request
    const [fundingRequestPDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('funding'),
        this.wallet.publicKey.toBuffer(),
        this.wallet.publicKey.toBuffer(), // payer
      ],
      this.programId
    );

    const targetAdminPubkey = new PublicKey(targetAdmin);
    
    const instruction = new TransactionInstruction({
      keys: [
        {
          pubkey: fundingRequestPDA,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: this.wallet.publicKey, // payer
          isSigner: true,
          isWritable: true,
        },
        {
          pubkey: this.wallet.publicKey, // user_wallet
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: SystemProgram.programId,
          isSigner: false,
          isWritable: false,
        },
      ],
      programId: this.programId,
      data: Buffer.concat([
        Buffer.from([181, 251, 230, 32, 73, 41, 179, 115]), // request_funding discriminator
        Buffer.alloc(8)
          .fill(0)
          .map((_, i) => (amount >> (i * 8)) & 0xff), // amount as u64
        targetAdminPubkey.toBuffer(), // target_admin as Pubkey (32 bytes)
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
      data: Buffer.concat([
        Buffer.from([CommandId.PUBLISH_PUBKEY]),
        this.wallet.publicKey.toBuffer(),
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

  // Отправка команды согласно W3B2 Bridge Protocol
  async dispatchCommand(
    commandId: number,
    mode: number,
    payload: Uint8Array,
    targetAdmin: string
  ): Promise<string> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');

    const transaction = new Transaction();
    const targetAdminPubkey = new PublicKey(targetAdmin);
    
    const data = Buffer.concat([
      Buffer.from([3]), // dispatch_command discriminator
      Buffer.alloc(8)
        .fill(0)
        .map((_, i) => (commandId >> (i * 8)) & 0xff), // command_id as u64
      Buffer.from([mode]), // mode as u8
      Buffer.alloc(4)
        .fill(0)
        .map((_, i) => (payload.length >> (i * 8)) & 0xff), // payload length as u32
      Buffer.from(payload), // payload
      targetAdminPubkey.toBuffer(), // target_admin as Pubkey
    ]);

    const instruction = new TransactionInstruction({
      keys: [
        {
          pubkey: this.wallet.publicKey, // authority - пользователь
          isSigner: true,
          isWritable: false,
        },
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
      this.saveWalletToStorage();
      console.log('🔑 Кошелек импортирован:', this.wallet.publicKey.toBase58());
      return true;
    } catch (error) {
      console.error('Ошибка импорта кошелька:', error);
      return false;
    }
  }

  // Сохранение кошелька в localStorage
  private saveWalletToStorage(): void {
    if (!this.wallet) return;
    
    try {
      const walletData = {
        publicKey: this.wallet.publicKey.toBase58(),
        privateKey: Buffer.from(this.wallet.secretKey).toString('base64'),
        timestamp: Date.now()
      };
      localStorage.setItem('w3b2_wallet', JSON.stringify(walletData));
      console.log('💾 Кошелек сохранен в localStorage');
    } catch (error) {
      console.error('Ошибка сохранения кошелька:', error);
    }
  }

  // Загрузка кошелька из localStorage
  private loadWalletFromStorage(): Keypair | null {
    try {
      const savedData = localStorage.getItem('w3b2_wallet');
      if (!savedData) return null;

      const walletData = JSON.parse(savedData);
      const secretKey = Buffer.from(walletData.privateKey, 'base64');
      const wallet = Keypair.fromSecretKey(secretKey);
      
      console.log('📂 Кошелек загружен из localStorage:', wallet.publicKey.toBase58());
      return wallet;
    } catch (error) {
      console.error('Ошибка загрузки кошелька:', error);
      return null;
    }
  }

  // Очистка кошелька из localStorage
  clearWalletFromStorage(): void {
    localStorage.removeItem('w3b2_wallet');
    console.log('🗑️ Кошелек удален из localStorage');
  }

  // Airdrop для тестовой сети
  async requestAirdrop(lamports: number = 1 * LAMPORTS_PER_SOL): Promise<string> {
    if (!this.wallet) throw new Error('Кошелек не инициализирован');

    try {
      const signature = await this.connection.requestAirdrop(
        this.wallet.publicKey,
        lamports
      );
      
      // Ждем подтверждения
      await this.connection.confirmTransaction(signature);
      
      console.log('💰 Airdrop получен:', signature);
      return signature;
    } catch (error) {
      console.error('Ошибка получения airdrop:', error);
      throw error;
    }
  }
}

// Экспортируем singleton instance
export const solanaService = new SolanaService();
export default solanaService;
