import { PublicKey } from '@solana/web3.js';

// API Response типы
export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

// Типы для команд W3B2
export const CommandMode = {
  RequestResponse: 0,
  OneWay: 1,
} as const;

export const CommandId = {
  PUBLISH_PUBKEY: 1,
  REQUEST_CONNECTION: 2,
  CRUD_CREATE: 5,
  CRUD_READ: 7,
  CRUD_UPDATE: 9,
  CRUD_DELETE: 11,
  START_SESSION: 13,
  END_SESSION: 15,
  ADMIN_AUTHORIZE: 17,
  ADMIN_ACCESS_CONTROL: 19,
  GENERIC_COMMAND: 21,
} as const;

export const FundingStatus = {
  Pending: 0,
  Approved: 1,
  Rejected: 2,
} as const;

export type CommandModeType = typeof CommandMode[keyof typeof CommandMode];
export type CommandIdType = typeof CommandId[keyof typeof CommandId];
export type FundingStatusType = typeof FundingStatus[keyof typeof FundingStatus];

export interface Destination {
  type: 'ipv4' | 'ipv6' | 'url';
  address: string;
  port?: number;
}

export interface CommandConfig {
  session_id: number;
  encrypted_session_key: Uint8Array;
  destination: Destination;
  meta: Uint8Array;
}

export interface FundingRequest {
  id?: string;
  user_wallet: PublicKey | string;
  amount: number;
  status: typeof FundingStatus[keyof typeof FundingStatus];
  target_admin: PublicKey | string;
  created_at: number;
  updated_at?: number;
}

export interface UserAccount {
  public_key: PublicKey | string;
  is_registered: boolean;
  created_at: number;
  last_activity: number;
}

export interface AdminAccount {
  public_key: PublicKey | string;
  is_active: boolean;
  created_at: number;
  funding_amount: number;
}

export interface SessionData {
  sessionId: number;
  serverPublicKey: PublicKey | string;
  expiresAt: number;
  isActive: boolean;
}

// Типы для CRUD операций
export interface CrudOperation {
  type: 'create' | 'read' | 'update' | 'delete';
  id?: string;
  data?: any;
  filters?: any;
}

export interface DatabaseRecord {
  id: string;
  data: any;
  created_at: number;
  updated_at: number;
  owner: PublicKey | string;
}

// Типы для статистики
export interface DatabaseStats {
  users: number;
  admins: number;
  fundingRequests: number;
  records: number;
}

export interface SessionStats {
  total: number;
  active: number;
  expired: number;
}

export interface AppStats {
  database: DatabaseStats;
  sessions: SessionStats;
}

// Типы для кошелька
export interface WalletInfo {
  publicKey: PublicKey | null;
  connected: boolean;
  connecting: boolean;
  disconnecting: boolean;
}

// Типы для форм
export interface RegisterUserForm {
  publicKey: string;
}

export interface RequestFundingForm {
  userWallet: string;
  amount: number;
  targetAdmin: string;
}

export interface CrudForm {
  operation: CrudOperation;
  owner: string;
}

// Типы для UI состояний
export interface LoadingState {
  isLoading: boolean;
  message?: string;
}

export interface ErrorState {
  hasError: boolean;
  message?: string;
  code?: string;
}

// Типы для навигации
export interface NavItem {
  label: string;
  href: string;
  icon?: React.ComponentType<any>;
  badge?: string | number;
}

// Типы для уведомлений
export interface Notification {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  title: string;
  message: string;
  duration?: number;
  action?: {
    label: string;
    onClick: () => void;
  };
}
