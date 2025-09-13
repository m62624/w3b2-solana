import { GrpcService } from '../services/grpcService';
import { BridgeEvent, CommandMode } from '../types/bridge.proto';

describe('GrpcService', () => {
  let grpcService: GrpcService;

  beforeEach(() => {
    grpcService = new GrpcService();
  });

  afterEach(async () => {
    await grpcService.stop();
  });

  test('должен создавать событие регистрации администратора', () => {
    const event = grpcService.createAdminRegisteredEvent('admin123', 1000);
    
    expect(event.adminRegistered).toBeDefined();
    expect(event.adminRegistered?.admin).toBe('admin123');
    expect(event.adminRegistered?.initialFunding).toBe(1000);
    expect(event.adminRegistered?.ts).toBeGreaterThan(0);
  });

  test('должен создавать событие регистрации пользователя', () => {
    const event = grpcService.createUserRegisteredEvent('user123', 500);
    
    expect(event.userRegistered).toBeDefined();
    expect(event.userRegistered?.user).toBe('user123');
    expect(event.userRegistered?.initialBalance).toBe(500);
    expect(event.userRegistered?.ts).toBeGreaterThan(0);
  });

  test('должен создавать событие запроса финансирования', () => {
    const event = grpcService.createFundingRequestedEvent('user123', 'admin456', 100);
    
    expect(event.fundingRequested).toBeDefined();
    expect(event.fundingRequested?.userWallet).toBe('user123');
    expect(event.fundingRequested?.targetAdmin).toBe('admin456');
    expect(event.fundingRequested?.amount).toBe(100);
    expect(event.fundingRequested?.ts).toBeGreaterThan(0);
  });

  test('должен создавать событие команды', () => {
    const payload = new Uint8Array([1, 2, 3, 4]);
    const event = grpcService.createCommandEvent('sender123', 'target456', 1, CommandMode.REQUEST_RESPONSE, payload);
    
    expect(event.commandEvent).toBeDefined();
    expect(event.commandEvent?.sender).toBe('sender123');
    expect(event.commandEvent?.target).toBe('target456');
    expect(event.commandEvent?.commandId).toBe(1);
    expect(event.commandEvent?.mode).toBe(CommandMode.REQUEST_RESPONSE);
    expect(event.commandEvent?.payload).toEqual(payload);
    expect(event.commandEvent?.ts).toBeGreaterThan(0);
  });

  test('должен возвращать количество активных подключений', () => {
    const count = grpcService.getActiveConnectionsCount();
    expect(count).toBe(0);
  });
});
