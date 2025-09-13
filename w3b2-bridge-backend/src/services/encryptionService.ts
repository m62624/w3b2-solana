import * as nacl from 'tweetnacl';
import { randomBytes } from 'crypto';
import { PublicKey } from '@solana/web3.js';
import { EncryptionKeys, SessionData, CommandConfig } from '../types/index.js';

export class EncryptionService {
  private serverKeys: EncryptionKeys;
  private activeSessions: Map<number, SessionData> = new Map();

  constructor() {
    // Генерируем ключи сервера
    const keyPair = nacl.box.keyPair();
    this.serverKeys = {
      publicKey: keyPair.publicKey,
      privateKey: keyPair.secretKey,
    };
    console.log('🔐 Ключи шифрования сервера сгенерированы');
  }

  getServerPublicKey(): Uint8Array {
    return this.serverKeys.publicKey;
  }

  getServerPublicKeyBase58(): string {
    return Buffer.from(this.serverKeys.publicKey).toString('base64');
  }

  // Гибридное шифрование: X25519 + AES-256
  encryptSessionKey(
    sessionKey: Uint8Array,
    clientPublicKey: Uint8Array
  ): Uint8Array {
    try {
      // Создаем временную пару ключей для этого сеанса
      const ephemeralKeyPair = nacl.box.keyPair();

      // Шифруем сессионный ключ с помощью X25519
      const nonce = randomBytes(24);
      const encrypted = nacl.box(
        sessionKey,
        nonce,
        clientPublicKey,
        ephemeralKeyPair.secretKey
      );

      if (!encrypted) {
        throw new Error('Ошибка шифрования сессионного ключа');
      }

      // Формат: [ephemeral_pubkey(32) | ciphertext(32) | tag(16)] = 80 bytes
      const result = new Uint8Array(80);
      result.set(ephemeralKeyPair.publicKey, 0);
      result.set(encrypted, 32);
      result.set(nonce, 64);

      return result;
    } catch (error) {
      console.error('❌ Ошибка шифрования сессионного ключа:', error);
      throw error;
    }
  }

  decryptSessionKey(
    encryptedSessionKey: Uint8Array,
    clientPublicKey: Uint8Array
  ): Uint8Array {
    try {
      if (encryptedSessionKey.length !== 80) {
        throw new Error('Неверный размер зашифрованного сессионного ключа');
      }

      const ciphertext = encryptedSessionKey.slice(32, 64);
      const nonce = encryptedSessionKey.slice(64, 80);

      const decrypted = nacl.box.open(
        ciphertext,
        nonce,
        clientPublicKey,
        this.serverKeys.privateKey
      );

      if (!decrypted) {
        throw new Error('Ошибка дешифрования сессионного ключа');
      }

      return decrypted;
    } catch (error) {
      console.error('❌ Ошибка дешифрования сессионного ключа:', error);
      throw error;
    }
  }

  // Симметричное шифрование с AES-256
  encryptData(data: Uint8Array, sessionKey: Uint8Array): Uint8Array {
    try {
      const nonce = randomBytes(24);
      const encrypted = nacl.secretbox(data, nonce, sessionKey);

      if (!encrypted) {
        throw new Error('Ошибка шифрования данных');
      }

      // Формат: [nonce(24) | encrypted_data]
      const result = new Uint8Array(24 + encrypted.length);
      result.set(nonce, 0);
      result.set(encrypted, 24);

      return result;
    } catch (error) {
      console.error('❌ Ошибка шифрования данных:', error);
      throw error;
    }
  }

  decryptData(encryptedData: Uint8Array, sessionKey: Uint8Array): Uint8Array {
    try {
      if (encryptedData.length < 24) {
        throw new Error('Неверный размер зашифрованных данных');
      }

      const nonce = encryptedData.slice(0, 24);
      const ciphertext = encryptedData.slice(24);

      const decrypted = nacl.secretbox.open(ciphertext, nonce, sessionKey);

      if (!decrypted) {
        throw new Error('Ошибка дешифрования данных');
      }

      return decrypted;
    } catch (error) {
      console.error('❌ Ошибка дешифрования данных:', error);
      throw error;
    }
  }

  // Создание новой сессии
  createSession(clientPublicKey: PublicKey): SessionData {
    const sessionId = Date.now();
    const sessionKey = randomBytes(32); // 256-bit AES key
    const expiresAt = Date.now() + 24 * 60 * 60 * 1000; // 24 часа

    const session: SessionData = {
      sessionId,
      sessionKey,
      clientPublicKey,
      serverPublicKey: new PublicKey(this.serverKeys.publicKey),
      isActive: true,
      createdAt: Date.now(),
      expiresAt,
    };

    this.activeSessions.set(sessionId, session);
    console.log(
      `🔑 Создана новая сессия ${sessionId} для клиента ${clientPublicKey.toBase58()}`
    );

    return session;
  }

  // Получение сессии
  getSession(sessionId: number): SessionData | undefined {
    const session = this.activeSessions.get(sessionId);

    if (session && session.isActive && Date.now() < session.expiresAt) {
      return session;
    }

    if (session) {
      this.activeSessions.delete(sessionId);
    }

    return undefined;
  }

  // Закрытие сессии
  closeSession(sessionId: number): boolean {
    const session = this.activeSessions.get(sessionId);
    if (session) {
      session.isActive = false;
      this.activeSessions.delete(sessionId);
      console.log(`🔒 Сессия ${sessionId} закрыта`);
      return true;
    }
    return false;
  }

  // Очистка истекших сессий
  cleanupExpiredSessions(): void {
    const now = Date.now();
    for (const [sessionId, session] of this.activeSessions.entries()) {
      if (!session.isActive || now >= session.expiresAt) {
        this.activeSessions.delete(sessionId);
      }
    }
  }

  // Шифрование CommandConfig
  encryptCommandConfig(
    config: CommandConfig,
    clientPublicKey: Uint8Array
  ): Uint8Array {
    try {
      // Создаем сессионный ключ
      const sessionKey = randomBytes(32);

      // Шифруем сессионный ключ
      const encryptedSessionKey = this.encryptSessionKey(
        sessionKey,
        clientPublicKey
      );

      // Создаем зашифрованную версию конфигурации
      const encryptedConfig: CommandConfig = {
        ...config,
        encrypted_session_key: encryptedSessionKey,
      };

      // Сериализуем конфигурацию (в реальном проекте используйте Borsh)
      const configData = this.serializeCommandConfig(encryptedConfig);

      // Шифруем данные конфигурации
      return this.encryptData(configData, sessionKey);
    } catch (error) {
      console.error('❌ Ошибка шифрования CommandConfig:', error);
      throw error;
    }
  }

  // Дешифрование CommandConfig
  decryptCommandConfig(
    encryptedConfig: Uint8Array
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    // _clientPublicKey: Uint8Array
  ): CommandConfig {
    try {
      // Сначала нужно получить сессионный ключ из зашифрованных данных
      // Это упрощенная версия - в реальном проекте нужна более сложная логика
      const sessionKey = randomBytes(32); // Временное решение

      // Дешифруем данные
      const configData = this.decryptData(encryptedConfig, sessionKey);

      // Десериализуем конфигурацию
      return this.deserializeCommandConfig(configData);
    } catch (error) {
      console.error('❌ Ошибка дешифрования CommandConfig:', error);
      throw error;
    }
  }

  // Простая сериализация CommandConfig (в реальном проекте используйте Borsh)
  private serializeCommandConfig(config: CommandConfig): Uint8Array {
    const buffer = Buffer.alloc(1024); // Максимальный размер
    let offset = 0;

    // session_id (8 bytes)
    buffer.writeBigUInt64LE(BigInt(config.session_id), offset);
    offset += 8;

    // encrypted_session_key (80 bytes)
    buffer.set(config.encrypted_session_key, offset);
    offset += 80;

    // destination
    const destType =
      config.destination.type === 'ipv4'
        ? 0
        : config.destination.type === 'ipv6'
          ? 1
          : 2;
    buffer.writeUInt8(destType, offset);
    offset += 1;

    if (config.destination.type === 'url') {
      const urlBytes = Buffer.from(config.destination.address, 'utf8');
      buffer.writeUInt32LE(urlBytes.length, offset);
      offset += 4;
      buffer.set(urlBytes, offset);
      offset += urlBytes.length;
    }

    // meta
    buffer.writeUInt32LE(config.meta.length, offset);
    offset += 4;
    buffer.set(config.meta, offset);
    offset += config.meta.length;

    return buffer.slice(0, offset);
  }

  // Простая десериализация CommandConfig
  private deserializeCommandConfig(data: Uint8Array): CommandConfig {
    const buffer = Buffer.from(data);
    let offset = 0;

    // session_id
    const session_id = Number(buffer.readBigUInt64LE(offset));
    offset += 8;

    // encrypted_session_key
    const encrypted_session_key = buffer.slice(offset, offset + 80);
    offset += 80;

    // destination
    const destType = buffer.readUInt8(offset);
    offset += 1;

    let destination: { type: 'ipv4' | 'ipv6' | 'url'; address: string };
    if (destType === 0 || destType === 1) {
      // IPv4/IPv6 - упрощенная реализация
      destination = {
        type: destType === 0 ? 'ipv4' : 'ipv6',
        address: '127.0.0.1',
      };
    } else {
      // URL
      const urlLength = buffer.readUInt32LE(offset);
      offset += 4;
      const url = buffer.slice(offset, offset + urlLength).toString('utf8');
      offset += urlLength;
      destination = { type: 'url', address: url };
    }

    // meta
    const metaLength = buffer.readUInt32LE(offset);
    offset += 4;
    const meta = buffer.slice(offset, offset + metaLength);
    offset += metaLength;

    return {
      session_id,
      encrypted_session_key,
      destination,
      meta,
    };
  }

  // Получение статистики сессий
  getSessionStats(): { total: number; active: number; expired: number } {
    const now = Date.now();
    let active = 0;
    let expired = 0;

    for (const session of this.activeSessions.values()) {
      if (session.isActive && now < session.expiresAt) {
        active++;
      } else {
        expired++;
      }
    }

    return {
      total: this.activeSessions.size,
      active,
      expired,
    };
  }
}
