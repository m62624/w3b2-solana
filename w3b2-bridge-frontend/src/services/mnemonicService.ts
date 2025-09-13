import * as bip39 from 'bip39';
import { Keypair } from '@solana/web3.js';

export interface MnemonicData {
  mnemonic: string;
  privateKey: Uint8Array;
  publicKey: string;
}

export class MnemonicService {
  /**
   * Генерирует новую мнемоническую фразу и соответствующие ключи
   */
  static generateMnemonic(): MnemonicData {
    try {
      // Генерируем 128 бит энтропии для 12 слов
      const mnemonic = bip39.generateMnemonic(128);
      
      // Получаем seed из мнемонической фразы
      const seed = bip39.mnemonicToSeedSync(mnemonic);
      
      // Генерируем ключи из seed (используем первые 32 байта)
      const privateKey = seed.slice(0, 32);
      const keypair = Keypair.fromSeed(privateKey);
      
      return {
        mnemonic,
        privateKey,
        publicKey: keypair.publicKey.toBase58(),
      };
    } catch (error) {
      console.error('❌ Ошибка генерации мнемонической фразы:', error);
      throw new Error('Не удалось сгенерировать мнемоническую фразу');
    }
  }

  /**
   * Восстанавливает ключи из мнемонической фразы
   */
  static restoreFromMnemonic(mnemonic: string): MnemonicData {
    try {
      // Проверяем валидность мнемонической фразы
      if (!bip39.validateMnemonic(mnemonic)) {
        throw new Error('Неверная мнемоническая фраза');
      }

      // Получаем seed из мнемонической фразы
      const seed = bip39.mnemonicToSeedSync(mnemonic);
      
      // Генерируем ключи из seed
      const privateKey = seed.slice(0, 32);
      const keypair = Keypair.fromSeed(privateKey);
      
      return {
        mnemonic,
        privateKey,
        publicKey: keypair.publicKey.toBase58(),
      };
    } catch (error) {
      console.error('❌ Ошибка восстановления из мнемонической фразы:', error);
      throw new Error('Не удалось восстановить ключи из мнемонической фразы');
    }
  }

  /**
   * Проверяет валидность мнемонической фразы
   */
  static validateMnemonic(mnemonic: string): boolean {
    return bip39.validateMnemonic(mnemonic);
  }

  /**
   * Получает список слов для автодополнения
   */
  static getWordList(): string[] {
    return bip39.wordlists.english;
  }

  /**
   * Проверяет, является ли слово валидным для BIP39
   */
  static isValidWord(word: string): boolean {
    return bip39.wordlists.english.includes(word.toLowerCase());
  }

  /**
   * Форматирует мнемоническую фразу для отображения
   */
  static formatMnemonic(mnemonic: string): string[] {
    return mnemonic.trim().split(/\s+/);
  }

  /**
   * Создает Keypair из мнемонической фразы
   */
  static createKeypairFromMnemonic(mnemonic: string): Keypair {
    const seed = bip39.mnemonicToSeedSync(mnemonic);
    const privateKey = seed.slice(0, 32);
    return Keypair.fromSeed(privateKey);
  }

  /**
   * Экспортирует приватный ключ в разных форматах
   */
  static exportPrivateKey(privateKey: Uint8Array, format: 'hex' | 'base64' | 'array' = 'hex'): string | number[] {
    switch (format) {
      case 'hex':
        return Array.from(privateKey)
          .map(b => b.toString(16).padStart(2, '0'))
          .join('');
      case 'base64':
        return Buffer.from(privateKey).toString('base64');
      case 'array':
        return Array.from(privateKey);
      default:
        throw new Error('Неподдерживаемый формат экспорта');
    }
  }

  /**
   * Импортирует приватный ключ из разных форматов
   */
  static importPrivateKey(data: string | number[], format: 'hex' | 'base64' | 'array' = 'hex'): Uint8Array {
    switch (format) {
      case 'hex':
        const hex = data as string;
        if (hex.length !== 64) {
          throw new Error('Неверная длина hex строки');
        }
        return new Uint8Array(hex.match(/.{2}/g)!.map(byte => parseInt(byte, 16)));
      case 'base64':
        return new Uint8Array(Buffer.from(data as string, 'base64'));
      case 'array':
        return new Uint8Array(data as number[]);
      default:
        throw new Error('Неподдерживаемый формат импорта');
    }
  }
}
