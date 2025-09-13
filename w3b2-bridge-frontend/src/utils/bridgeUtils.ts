import { PublicKey, Connection } from '@solana/web3.js';
import { BridgeClient } from '../services/bridgeClient';
import { CommandMode, FundingStatus, type Destination } from '../types/bridge';

/**
 * Утилиты для работы с Bridge клиентом
 */
export class BridgeUtils {
  private client: BridgeClient;
  private connection: Connection;

  constructor(connection: Connection) {
    this.connection = connection;
    this.client = new BridgeClient(connection);
  }

  /**
   * Проверить, зарегистрирован ли админ
   */
  async isAdminRegistered(coSigner: PublicKey): Promise<boolean> {
    try {
      const [adminPDA] = await this.client.getAdminAccountPDA(coSigner);
      const adminAccount = await this.client.getAdminAccount(adminPDA);
      return adminAccount !== null && adminAccount.meta.active;
    } catch (error) {
      console.error('Ошибка при проверке регистрации админа:', error);
      return false;
    }
  }

  /**
   * Проверить, зарегистрирован ли пользователь
   */
  async isUserRegistered(coSigner: PublicKey): Promise<boolean> {
    try {
      const [userPDA] = await this.client.getUserAccountPDA(coSigner);
      const userAccount = await this.client.getUserAccount(userPDA);
      return userAccount !== null && userAccount.meta.active;
    } catch (error) {
      console.error('Ошибка при проверке регистрации пользователя:', error);
      return false;
    }
  }

  /**
   * Получить баланс админа
   */
  async getAdminBalance(coSigner: PublicKey): Promise<number> {
    try {
      const [adminPDA] = await this.client.getAdminAccountPDA(coSigner);
      const accountInfo = await this.connection.getAccountInfo(adminPDA);
      return accountInfo ? accountInfo.lamports : 0;
    } catch (error) {
      console.error('Ошибка при получении баланса админа:', error);
      return 0;
    }
  }

  /**
   * Получить баланс пользователя
   */
  async getUserBalance(coSigner: PublicKey): Promise<number> {
    try {
      const [userPDA] = await this.client.getUserAccountPDA(coSigner);
      const accountInfo = await this.connection.getAccountInfo(userPDA);
      return accountInfo ? accountInfo.lamports : 0;
    } catch (error) {
      console.error('Ошибка при получении баланса пользователя:', error);
      return 0;
    }
  }

  /**
   * Получить все активные запросы финансирования для админа
   */
  async getPendingFundingRequests(targetAdmin: PublicKey): Promise<Array<{
    requestPDA: PublicKey;
    userWallet: PublicKey;
    amount: number;
    status: FundingStatus;
  }>> {
    try {
      // В реальной реализации здесь нужно сканировать все аккаунты программы
      // и фильтровать по targetAdmin и статусу Pending
      // Для упрощения возвращаем пустой массив
      return [];
    } catch (error) {
      console.error('Ошибка при получении запросов финансирования:', error);
      return [];
    }
  }

  /**
   * Создать дестинацию для IPv4
   */
  createIPv4Destination(ip: string, port: number): Destination {
    const parts = ip.split('.').map(Number);
    if (parts.length !== 4 || parts.some(p => p < 0 || p > 255)) {
      throw new Error('Неверный IPv4 адрес');
    }
    
    return {
      type: 'ipv4',
      address: parts as [number, number, number, number],
      port,
    };
  }

  /**
   * Создать дестинацию для IPv6
   */
  createIPv6Destination(ip: string, port: number): Destination {
    // Упрощенная реализация - в реальности нужно парсить IPv6
    const parts = new Array(16).fill(0);
    return {
      type: 'ipv6',
      address: parts as [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number],
      port,
    };
  }

  /**
   * Создать дестинацию для URL
   */
  createURLDestination(url: string): Destination {
    try {
      new URL(url); // Валидация URL
      return {
        type: 'url',
        url,
      };
    } catch (error) {
      throw new Error('Неверный URL');
    }
  }

  /**
   * Генерировать случайный сессионный ключ
   */
  generateSessionKey(): Uint8Array {
    return crypto.getRandomValues(new Uint8Array(32));
  }

  /**
   * Генерировать случайный ID сессии
   */
  generateSessionId(): number {
    return Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
  }

  /**
   * Конвертировать SOL в lamports
   */
  solToLamports(sol: number): number {
    return Math.floor(sol * 1_000_000_000);
  }

  /**
   * Конвертировать lamports в SOL
   */
  lamportsToSol(lamports: number): number {
    return lamports / 1_000_000_000;
  }

  /**
   * Форматировать баланс для отображения
   */
  formatBalance(lamports: number, decimals: number = 4): string {
    const sol = this.lamportsToSol(lamports);
    return sol.toFixed(decimals) + ' SOL';
  }

  /**
   * Валидировать адрес Solana
   */
  isValidSolanaAddress(address: string): boolean {
    try {
      new PublicKey(address);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Получить статус запроса финансирования в текстовом виде
   */
  getFundingStatusText(status: FundingStatus): string {
    switch (status) {
      case FundingStatus.Pending:
        return 'Ожидает';
      case FundingStatus.Approved:
        return 'Одобрен';
      case FundingStatus.Rejected:
        return 'Отклонен';
      default:
        return 'Неизвестно';
    }
  }

  /**
   * Получить режим команды в текстовом виде
   */
  getCommandModeText(mode: CommandMode): string {
    switch (mode) {
      case CommandMode.RequestResponse:
        return 'Запрос-Ответ';
      case CommandMode.OneWay:
        return 'Односторонняя';
      default:
        return 'Неизвестно';
    }
  }

  /**
   * Создать зашифрованный сессионный ключ (заглушка)
   * В реальной реализации здесь должна быть гибридная шифровка X25519+AEAD
   */
  createEncryptedSessionKey(sessionKey: Uint8Array, recipientPublicKey: PublicKey): Uint8Array {
    // Заглушка - возвращаем 80 байт нулей
    // В реальности: [ephemeral_pubkey(32) | ciphertext(32) | tag(16)] = 80 bytes
    return new Uint8Array(80);
  }

  /**
   * Расшифровать сессионный ключ (заглушка)
   */
  decryptSessionKey(encryptedKey: Uint8Array, privateKey: Uint8Array): Uint8Array {
    // Заглушка - возвращаем случайный ключ
    return this.generateSessionKey();
  }
}

/**
 * Глобальные утилиты
 */
export const bridgeUtils = {
  /**
   * Создать экземпляр утилит
   */
  create(connection: Connection): BridgeUtils {
    return new BridgeUtils(connection);
  },

  /**
   * Быстрая проверка адреса
   */
  isValidAddress(address: string): boolean {
    try {
      new PublicKey(address);
      return true;
    } catch {
      return false;
    }
  },

  /**
   * Форматировать SOL
   */
  formatSOL(lamports: number, decimals: number = 4): string {
    return (lamports / 1_000_000_000).toFixed(decimals) + ' SOL';
  },

  /**
   * Конвертировать SOL в lamports
   */
  toLamports(sol: number): number {
    return Math.floor(sol * 1_000_000_000);
  },

  /**
   * Конвертировать lamports в SOL
   */
  toSOL(lamports: number): number {
    return lamports / 1_000_000_000;
  },
};
