// Generated types from bridge.proto

export interface Empty {}

export interface AdminRegistered {
  admin: string;
  initialFunding: number;
  ts: number;
}

export interface UserRegistered {
  user: string;
  initialBalance: number;
  ts: number;
}

export interface AdminDeactivated {
  admin: string;
  ts: number;
}

export interface UserDeactivated {
  user: string;
  ts: number;
}

export interface FundingRequested {
  userWallet: string;
  targetAdmin: string;
  amount: number;
  ts: number;
}

export interface FundingApproved {
  userWallet: string;
  approvedBy: string;
  amount: number;
  ts: number;
}

export enum CommandMode {
  REQUEST_RESPONSE = 0,
  ONE_WAY = 1,
}

export interface CommandEvent {
  sender: string;
  target: string;
  commandId: number;
  mode: CommandMode;
  payload: Uint8Array;
  ts: number;
}

export interface BridgeEvent {
  adminRegistered?: AdminRegistered;
  userRegistered?: UserRegistered;
  adminDeactivated?: AdminDeactivated;
  userDeactivated?: UserDeactivated;
  fundingRequested?: FundingRequested;
  fundingApproved?: FundingApproved;
  commandEvent?: CommandEvent;
}

// gRPC Service definitions
export interface BridgeService {
  streamEvents(request: Empty): AsyncIterable<BridgeEvent>;
}

// gRPC Client interface
export interface BridgeServiceClient {
  streamEvents(request: Empty): AsyncIterable<BridgeEvent>;
}
