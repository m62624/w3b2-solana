import { PublicKey } from '@solana/web3.js';

export enum CommandMode {
  RequestResponse = 0,
  OneWay = 1,
}

export enum CommandId {
  PUBLISH_PUBKEY = 1,
  REQUEST_CONNECTION = 2,
  CRUD_CREATE = 5,
  CRUD_READ = 7,
  CRUD_UPDATE = 9,
  CRUD_DELETE = 11,
  START_SESSION = 13,
  END_SESSION = 15,
  ADMIN_AUTHORIZE = 17,
  ADMIN_ACCESS_CONTROL = 19,
  GENERIC_COMMAND = 21,
}

export enum FundingStatus {
  Pending = 0,
  Approved = 1,
  Rejected = 2,
}

export interface Destination {
  type: 'ipv4' | 'ipv6' | 'url';
  address: string;
  port?: number;
}

export interface CommandConfig {
  session_id: number;
  encrypted_session_key: Uint8Array; // 80 bytes - фиксированный размер
  destination: Destination;
  meta: Uint8Array;
}

export interface CommandRecord {
  sender: PublicKey;
  command_id: number;
  mode: CommandMode;
  payload: Uint8Array;
}

export interface FundingRequest {
  user_wallet: PublicKey;
  amount: number;
  status: FundingStatus;
  target_admin: PublicKey;
  created_at: number;
}

export interface UserAccount {
  public_key: PublicKey;
  is_registered: boolean;
  created_at: number;
  last_activity: number;
}

export interface AdminAccount {
  public_key: PublicKey;
  is_active: boolean;
  created_at: number;
  funding_amount: number;
}

// API Response типы
export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

export interface BlockchainEvent {
  type: string;
  data: any;
  signature: string;
  slot: number;
  timestamp: number;
}

// Типы для шифрования
export interface EncryptionKeys {
  publicKey: Uint8Array;
  privateKey: Uint8Array;
}

export interface SessionData {
  sessionId: number;
  sessionKey: Uint8Array;
  clientPublicKey: PublicKey;
  serverPublicKey: PublicKey;
  isActive: boolean;
  createdAt: number;
  expiresAt: number;
}

// Типы для базы данных
export interface DatabaseRecord {
  id: string;
  data: any;
  created_at: number;
  updated_at: number;
  owner: PublicKey;
}

export interface CrudOperation {
  type: 'create' | 'read' | 'update' | 'delete';
  id?: string;
  data?: any;
  filters?: any;
}
