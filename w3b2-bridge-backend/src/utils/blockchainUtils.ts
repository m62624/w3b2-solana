import { PublicKey } from '@solana/web3.js';
import { Destination, CommandConfig } from '../types/index.js';

// Типы для blockchain (соответствуют Rust структурам)
export interface BlockchainDestination {
  type: 'ipv4' | 'ipv6' | 'url';
  ipv4?: [number, number, number, number];
  ipv6?: [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number];
  port?: number;
  url?: string;
}

export interface BlockchainCommandConfig {
  session_id: number;
  encrypted_session_key: number[]; // 80 bytes как массив чисел
  destination: BlockchainDestination;
  meta: number[];
}

/**
 * Конвертирует Destination из backend формата в blockchain формат
 */
export function convertDestinationToBlockchain(dest: Destination): BlockchainDestination {
  switch (dest.type) {
    case 'ipv4': {
      const parts = dest.address.split('.').map(Number);
      if (parts.length !== 4 || parts.some(p => isNaN(p) || p < 0 || p > 255)) {
        throw new Error('Неверный IPv4 адрес');
      }
      return {
        type: 'ipv4',
        ipv4: [parts[0], parts[1], parts[2], parts[3]],
        port: dest.port || 80,
      };
    }
    
    case 'ipv6': {
      // Упрощенная конвертация IPv6 - в реальном проекте нужна более сложная логика
      const parts = dest.address.split(':').map(part => {
        const num = parseInt(part, 16);
        return isNaN(num) ? 0 : num;
      });
      
      // Дополняем до 8 частей
      while (parts.length < 8) {
        parts.push(0);
      }
      
      const ipv6: [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number] = [
        (parts[0] >> 8) & 0xFF, parts[0] & 0xFF,
        (parts[1] >> 8) & 0xFF, parts[1] & 0xFF,
        (parts[2] >> 8) & 0xFF, parts[2] & 0xFF,
        (parts[3] >> 8) & 0xFF, parts[3] & 0xFF,
        (parts[4] >> 8) & 0xFF, parts[4] & 0xFF,
        (parts[5] >> 8) & 0xFF, parts[5] & 0xFF,
        (parts[6] >> 8) & 0xFF, parts[6] & 0xFF,
        (parts[7] >> 8) & 0xFF, parts[7] & 0xFF,
      ];
      
      return {
        type: 'ipv6',
        ipv6,
        port: dest.port || 80,
      };
    }
    
    case 'url':
      return {
        type: 'url',
        url: dest.address,
      };
    
    default:
      throw new Error('Неподдерживаемый тип назначения');
  }
}

/**
 * Конвертирует Destination из blockchain формата в backend формат
 */
export function convertDestinationFromBlockchain(dest: BlockchainDestination): Destination {
  switch (dest.type) {
    case 'ipv4':
      if (!dest.ipv4) {
        throw new Error('Отсутствуют данные IPv4');
      }
      return {
        type: 'ipv4',
        address: dest.ipv4.join('.'),
        port: dest.port,
      };
    
    case 'ipv6':
      if (!dest.ipv6) {
        throw new Error('Отсутствуют данные IPv6');
      }
      // Конвертируем обратно в строку IPv6
      const parts: string[] = [];
      for (let i = 0; i < 8; i++) {
        const part = (dest.ipv6[i * 2] << 8) | dest.ipv6[i * 2 + 1];
        parts.push(part.toString(16));
      }
      return {
        type: 'ipv6',
        address: parts.join(':'),
        port: dest.port,
      };
    
    case 'url':
      if (!dest.url) {
        throw new Error('Отсутствует URL');
      }
      return {
        type: 'url',
        address: dest.url,
      };
    
    default:
      throw new Error('Неподдерживаемый тип назначения');
  }
}

/**
 * Конвертирует CommandConfig из backend формата в blockchain формат
 */
export function convertCommandConfigToBlockchain(config: CommandConfig): BlockchainCommandConfig {
  if (config.encrypted_session_key.length !== 80) {
    throw new Error('encrypted_session_key должен быть 80 байт');
  }

  return {
    session_id: config.session_id,
    encrypted_session_key: Array.from(config.encrypted_session_key),
    destination: convertDestinationToBlockchain(config.destination),
    meta: Array.from(config.meta),
  };
}

/**
 * Конвертирует CommandConfig из blockchain формата в backend формат
 */
export function convertCommandConfigFromBlockchain(config: BlockchainCommandConfig): CommandConfig {
  if (config.encrypted_session_key.length !== 80) {
    throw new Error('encrypted_session_key должен быть 80 байт');
  }

  return {
    session_id: config.session_id,
    encrypted_session_key: new Uint8Array(config.encrypted_session_key),
    destination: convertDestinationFromBlockchain(config.destination),
    meta: new Uint8Array(config.meta),
  };
}

/**
 * Создает Borsh сериализацию для CommandConfig
 */
export function serializeCommandConfig(config: CommandConfig): Uint8Array {
  const blockchainConfig = convertCommandConfigToBlockchain(config);
  
  // Простая сериализация - в реальном проекте используйте Borsh
  const buffer = new ArrayBuffer(1024);
  const view = new DataView(buffer);
  let offset = 0;

  // session_id (8 bytes)
  view.setBigUint64(offset, BigInt(blockchainConfig.session_id), true);
  offset += 8;

  // encrypted_session_key (80 bytes)
  for (let i = 0; i < 80; i++) {
    view.setUint8(offset + i, blockchainConfig.encrypted_session_key[i]);
  }
  offset += 80;

  // destination type (1 byte)
  const destType = blockchainConfig.destination.type === 'ipv4' ? 0 : 
                   blockchainConfig.destination.type === 'ipv6' ? 1 : 2;
  view.setUint8(offset, destType);
  offset += 1;

  // destination data
  if (blockchainConfig.destination.type === 'ipv4' && blockchainConfig.destination.ipv4) {
    for (let i = 0; i < 4; i++) {
      view.setUint8(offset + i, blockchainConfig.destination.ipv4[i]);
    }
    offset += 4;
    view.setUint16(offset, blockchainConfig.destination.port || 80, true);
    offset += 2;
  } else if (blockchainConfig.destination.type === 'ipv6' && blockchainConfig.destination.ipv6) {
    for (let i = 0; i < 16; i++) {
      view.setUint8(offset + i, blockchainConfig.destination.ipv6[i]);
    }
    offset += 16;
    view.setUint16(offset, blockchainConfig.destination.port || 80, true);
    offset += 2;
  } else if (blockchainConfig.destination.type === 'url' && blockchainConfig.destination.url) {
    const urlBytes = new TextEncoder().encode(blockchainConfig.destination.url);
    view.setUint32(offset, urlBytes.length, true);
    offset += 4;
    for (let i = 0; i < urlBytes.length; i++) {
      view.setUint8(offset + i, urlBytes[i]);
    }
    offset += urlBytes.length;
  }

  // meta length (4 bytes) + data
  view.setUint32(offset, blockchainConfig.meta.length, true);
  offset += 4;
  for (let i = 0; i < blockchainConfig.meta.length; i++) {
    view.setUint8(offset + i, blockchainConfig.meta[i]);
  }
  offset += blockchainConfig.meta.length;

  return new Uint8Array(buffer, 0, offset);
}

/**
 * Десериализует CommandConfig из Borsh
 */
export function deserializeCommandConfig(data: Uint8Array): CommandConfig {
  const view = new DataView(data.buffer, data.byteOffset);
  let offset = 0;

  // session_id
  const session_id = Number(view.getBigUint64(offset, true));
  offset += 8;

  // encrypted_session_key
  const encrypted_session_key = new Uint8Array(80);
  for (let i = 0; i < 80; i++) {
    encrypted_session_key[i] = view.getUint8(offset + i);
  }
  offset += 80;

  // destination type
  const destType = view.getUint8(offset);
  offset += 1;

  let destination: Destination;
  if (destType === 0) {
    // IPv4
    const ipv4: [number, number, number, number] = [
      view.getUint8(offset),
      view.getUint8(offset + 1),
      view.getUint8(offset + 2),
      view.getUint8(offset + 3),
    ];
    offset += 4;
    const port = view.getUint16(offset, true);
    offset += 2;
    
    destination = {
      type: 'ipv4',
      address: ipv4.join('.'),
      port,
    };
  } else if (destType === 1) {
    // IPv6
    const ipv6: number[] = [];
    for (let i = 0; i < 16; i++) {
      ipv6.push(view.getUint8(offset + i));
    }
    offset += 16;
    const port = view.getUint16(offset, true);
    offset += 2;
    
    // Конвертируем обратно в строку IPv6
    const parts: string[] = [];
    for (let i = 0; i < 8; i++) {
      const part = (ipv6[i * 2] << 8) | ipv6[i * 2 + 1];
      parts.push(part.toString(16));
    }
    
    destination = {
      type: 'ipv6',
      address: parts.join(':'),
      port,
    };
  } else {
    // URL
    const urlLength = view.getUint32(offset, true);
    offset += 4;
    const urlBytes = new Uint8Array(data.buffer, data.byteOffset + offset, urlLength);
    const url = new TextDecoder().decode(urlBytes);
    offset += urlLength;
    
    destination = {
      type: 'url',
      address: url,
    };
  }

  // meta
  const metaLength = view.getUint32(offset, true);
  offset += 4;
  const meta = new Uint8Array(data.buffer, data.byteOffset + offset, metaLength);

  return {
    session_id,
    encrypted_session_key,
    destination,
    meta,
  };
}
