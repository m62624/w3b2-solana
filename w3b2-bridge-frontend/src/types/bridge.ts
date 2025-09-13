import { PublicKey } from '@solana/web3.js';

export const PROGRAM_ID = new PublicKey('3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr');

export const CommandMode = {
  RequestResponse: 0,
  OneWay: 1,
} as const;

export const FundingStatus = {
  Pending: 0,
  Approved: 1,
  Rejected: 2,
} as const;

// Типы для дестинации
export interface Destination {
  type: 'ipv4' | 'ipv6' | 'url';
  address?: [number, number, number, number] | [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number];
  port?: number;
  url?: string;
}

// Конфигурация команды
export interface CommandConfig {
  sessionId: number;
  encryptedSessionKey: Uint8Array; // 80 bytes
  destination: Destination;
  meta: Uint8Array;
}

// Запись команды
export interface CommandRecord {
  sender: PublicKey;
  commandId: number;
  mode: typeof CommandMode[keyof typeof CommandMode];
  payload: Uint8Array;
}

// Метаданные аккаунта
export interface AccountMeta {
  owner: PublicKey;
  coSigner: PublicKey;
  active: boolean;
}

// Админский аккаунт
export interface AdminAccount {
  meta: AccountMeta;
}

// Пользовательский аккаунт
export interface UserAccount {
  meta: AccountMeta;
}

// Запрос на финансирование
export interface FundingRequest {
  userWallet: PublicKey;
  targetAdmin: PublicKey;
  amount: number;
  status: typeof FundingStatus[keyof typeof FundingStatus];
}

// События программы
export interface AdminRegisteredEvent {
  admin: PublicKey;
  initialFunding: number;
  ts: number;
}

export interface UserRegisteredEvent {
  user: PublicKey;
  initialBalance: number;
  ts: number;
}

export interface AdminDeactivatedEvent {
  admin: PublicKey;
  ts: number;
}

export interface UserDeactivatedEvent {
  user: PublicKey;
  ts: number;
}

export interface FundingRequestedEvent {
  userWallet: PublicKey;
  targetAdmin: PublicKey;
  amount: number;
  ts: number;
}

export interface FundingApprovedEvent {
  userWallet: PublicKey;
  approvedBy: PublicKey;
  amount: number;
  ts: number;
}

export interface CommandEvent {
  sender: PublicKey;
  target: PublicKey;
  commandId: number;
  mode: typeof CommandMode[keyof typeof CommandMode];
  payload: Uint8Array;
  ts: number;
}

// Ошибки программы
export const BridgeError = {
  Unauthorized: 'Admin is not authorized to approve this request',
  AlreadyRegistered: 'PDA already registered for this owner',
  PayloadTooLarge: 'Payload too large',
  RequestAlreadyProcessed: 'Funding request has already been processed',
  InsufficientFundsForFunding: 'Insufficient funds in admin profile to approve funding request',
  InsufficientFundsForAdmin: 'Insufficient funds to create admin profile PDA',
  InactiveAccount: 'Account is inactive',
} as const;

// Константы команд
export const CMD_PUBLISH_PUBKEY = 1;
export const CMD_REQUEST_CONNECTION = 2;

// Интерфейс для параметров регистрации
export interface RegisterAdminParams {
  payer: PublicKey;
  authority: PublicKey;
  coSigner: PublicKey;
  fundingAmount: number;
}

export interface RegisterUserParams {
  payer: PublicKey;
  userWallet: PublicKey;
  coSigner: PublicKey;
  initialBalance: number;
}

export interface RequestFundingParams {
  payer: PublicKey;
  userAccount: PublicKey;
  amount: number;
  targetAdmin: PublicKey;
}

export interface ApproveFundingParams {
  adminAuthority: PublicKey;
  fundingRequest: PublicKey;
  userWallet: PublicKey;
}

export interface DispatchCommandParams {
  authority: PublicKey;
  targetPubkey: PublicKey;
  commandId: number;
  mode: typeof CommandMode[keyof typeof CommandMode];
  payload: Uint8Array;
}
