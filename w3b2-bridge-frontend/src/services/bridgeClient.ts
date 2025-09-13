import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  type TransactionSignature,
  Keypair,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
  PROGRAM_ID,
  type RegisterAdminParams,
  type RegisterUserParams,
  type RequestFundingParams,
  type ApproveFundingParams,
  type DispatchCommandParams,
  CommandMode,
  FundingStatus,
  type CommandConfig,
  type Destination,
  CMD_PUBLISH_PUBKEY,
  CMD_REQUEST_CONNECTION,
} from '../types/bridge';

export class BridgeClient {
  private connection: Connection;
  private programId: PublicKey;

  constructor(connection: Connection, programId: PublicKey = PROGRAM_ID) {
    this.connection = connection;
    this.programId = programId;
  }

  /**
   * Получить PDA для админского аккаунта
   */
  async getAdminAccountPDA(coSigner: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('admin'), coSigner.toBuffer()],
      this.programId
    );
  }

  /**
   * Получить PDA для пользовательского аккаунта
   */
  async getUserAccountPDA(coSigner: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('user'), coSigner.toBuffer()],
      this.programId
    );
  }

  /**
   * Получить PDA для запроса финансирования
   */
  async getFundingRequestPDA(userAccount: PublicKey, payer: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('funding'), userAccount.toBuffer(), payer.toBuffer()],
      this.programId
    );
  }

  /**
   * Регистрация админа
   */
  async registerAdmin(
    params: RegisterAdminParams,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const [adminAccountPDA] = await this.getAdminAccountPDA(params.coSigner);
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: adminAccountPDA, isSigner: false, isWritable: true },
        { pubkey: params.payer, isSigner: true, isWritable: true },
        { pubkey: params.authority, isSigner: true, isWritable: false },
        { pubkey: params.coSigner, isSigner: true, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data: this.encodeRegisterAdminData(params.initialBalance),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Регистрация пользователя
   */
  async registerUser(
    params: RegisterUserParams,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const [userAccountPDA] = await this.getUserAccountPDA(params.coSigner);
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: userAccountPDA, isSigner: false, isWritable: true },
        { pubkey: params.payer, isSigner: true, isWritable: true },
        { pubkey: params.userWallet, isSigner: true, isWritable: false },
        { pubkey: params.coSigner, isSigner: true, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data: this.encodeRegisterUserData(params.initialBalance),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Деактивация админа
   */
  async deactivateAdmin(
    adminAccount: PublicKey,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: adminAccount, isSigner: false, isWritable: true },
      ],
      programId: this.programId,
      data: this.encodeDeactivateAdminData(),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Деактивация пользователя
   */
  async deactivateUser(
    userAccount: PublicKey,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: userAccount, isSigner: false, isWritable: true },
      ],
      programId: this.programId,
      data: this.encodeDeactivateUserData(),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Запрос финансирования
   */
  async requestFunding(
    params: RequestFundingParams,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const [fundingRequestPDA] = await this.getFundingRequestPDA(params.userAccount, params.payer);
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: fundingRequestPDA, isSigner: false, isWritable: true },
        { pubkey: params.payer, isSigner: true, isWritable: true },
        { pubkey: params.userAccount, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data: this.encodeRequestFundingData(params.amount, params.targetAdmin),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Одобрение финансирования
   */
  async approveFunding(
    params: ApproveFundingParams,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const [adminAccountPDA] = await this.getAdminAccountPDA(params.adminAuthority);
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: adminAccountPDA, isSigner: false, isWritable: true },
        { pubkey: params.fundingRequest, isSigner: false, isWritable: true },
        { pubkey: params.userWallet, isSigner: false, isWritable: true },
        { pubkey: params.adminAuthority, isSigner: true, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data: this.encodeApproveFundingData(),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Отправка команды от админа
   */
  async dispatchCommandAdmin(
    params: DispatchCommandParams,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const [adminAccountPDA] = await this.getAdminAccountPDA(params.authority);
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: params.authority, isSigner: true, isWritable: false },
        { pubkey: adminAccountPDA, isSigner: false, isWritable: true },
      ],
      programId: this.programId,
      data: this.encodeDispatchCommandData(
        params.commandId,
        params.mode,
        params.payload,
        params.targetPubkey
      ),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Отправка команды от пользователя
   */
  async dispatchCommandUser(
    params: DispatchCommandParams,
    signers: Keypair[]
  ): Promise<TransactionSignature> {
    const [userAccountPDA] = await this.getUserAccountPDA(params.authority);
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: params.authority, isSigner: true, isWritable: false },
        { pubkey: userAccountPDA, isSigner: false, isWritable: true },
      ],
      programId: this.programId,
      data: this.encodeDispatchCommandData(
        params.commandId,
        params.mode,
        params.payload,
        params.targetPubkey
      ),
    });

    const transaction = new Transaction().add(instruction);
    return await sendAndConfirmTransaction(this.connection, transaction, signers);
  }

  /**
   * Получить данные админского аккаунта
   */
  async getAdminAccount(adminAccount: PublicKey): Promise<any | null> {
    try {
      const accountInfo = await this.connection.getAccountInfo(adminAccount);
      if (!accountInfo) return null;
      
      return this.decodeAdminAccount(accountInfo.data);
    } catch (error) {
      console.error('Ошибка при получении админского аккаунта:', error);
      return null;
    }
  }

  /**
   * Получить данные пользовательского аккаунта
   */
  async getUserAccount(userAccount: PublicKey): Promise<any | null> {
    try {
      const accountInfo = await this.connection.getAccountInfo(userAccount);
      if (!accountInfo) return null;
      
      return this.decodeUserAccount(accountInfo.data);
    } catch (error) {
      console.error('Ошибка при получении пользовательского аккаунта:', error);
      return null;
    }
  }

  /**
   * Получить данные запроса финансирования
   */
  async getFundingRequest(fundingRequest: PublicKey): Promise<any | null> {
    try {
      const accountInfo = await this.connection.getAccountInfo(fundingRequest);
      if (!accountInfo) return null;
      
      return this.decodeFundingRequest(accountInfo.data);
    } catch (error) {
      console.error('Ошибка при получении запроса финансирования:', error);
      return null;
    }
  }

  /**
   * Подписать транзакцию (для использования с кошельками)
   */
  async signTransaction(transaction: Transaction, signers: Keypair[]): Promise<Transaction> {
    transaction.partialSign(...signers);
    return transaction;
  }

  // Кодирование данных инструкций
  private encodeRegisterAdminData(initialBalance: number): Buffer {
    const data = Buffer.alloc(8 + 8); // discriminator + u64
    data.writeUInt32LE(0, 0); // discriminator для register_admin
    data.writeBigUInt64LE(BigInt(initialBalance), 8);
    return data;
  }

  private encodeRegisterUserData(initialBalance: number): Buffer {
    const data = Buffer.alloc(8 + 8); // discriminator + u64
    data.writeUInt32LE(1, 0); // discriminator для register_user
    data.writeBigUInt64LE(BigInt(initialBalance), 8);
    return data;
  }

  private encodeDeactivateAdminData(): Buffer {
    const data = Buffer.alloc(8);
    data.writeUInt32LE(2, 0); // discriminator для deactivate_admin
    return data;
  }

  private encodeDeactivateUserData(): Buffer {
    const data = Buffer.alloc(8);
    data.writeUInt32LE(3, 0); // discriminator для deactivate_user
    return data;
  }

  private encodeRequestFundingData(amount: number, targetAdmin: PublicKey): Buffer {
    const data = Buffer.alloc(8 + 8 + 32); // discriminator + u64 + pubkey
    data.writeUInt32LE(4, 0); // discriminator для request_funding
    data.writeBigUInt64LE(BigInt(amount), 8);
    data.set(targetAdmin.toBuffer(), 16);
    return data;
  }

  private encodeApproveFundingData(): Buffer {
    const data = Buffer.alloc(8);
    data.writeUInt32LE(5, 0); // discriminator для approve_funding
    return data;
  }

  private encodeDispatchCommandData(
    commandId: number,
    mode: typeof CommandMode[keyof typeof CommandMode],
    payload: Uint8Array,
    targetPubkey: PublicKey
  ): Buffer {
    const payloadLength = payload.length;
    const data = Buffer.alloc(8 + 8 + 1 + 4 + payloadLength + 32); // discriminator + u64 + u8 + u32 + payload + pubkey
    let offset = 0;
    
    data.writeUInt32LE(6, offset); // discriminator для dispatch_command_admin/user
    offset += 8;
    
    data.writeBigUInt64LE(BigInt(commandId), offset);
    offset += 8;
    
    data.writeUInt8(mode, offset);
    offset += 1;
    
    data.writeUInt32LE(payloadLength, offset);
    offset += 4;
    
    data.set(payload, offset);
    offset += payloadLength;
    
    data.set(targetPubkey.toBuffer(), offset);
    
    return data;
  }

  // Декодирование данных аккаунтов
  private decodeAdminAccount(data: Buffer): any {
    const offset = 8; // пропускаем discriminator
    const owner = new PublicKey(data.slice(offset, offset + 32));
    const coSigner = new PublicKey(data.slice(offset + 32, offset + 64));
    const active = data[offset + 64] === 1;
    
    return {
      meta: {
        owner,
        coSigner,
        active,
      },
    };
  }

  private decodeUserAccount(data: Buffer): any {
    const offset = 8; // пропускаем discriminator
    const owner = new PublicKey(data.slice(offset, offset + 32));
    const coSigner = new PublicKey(data.slice(offset + 32, offset + 64));
    const active = data[offset + 64] === 1;
    
    return {
      meta: {
        owner,
        coSigner,
        active,
      },
    };
  }

  private decodeFundingRequest(data: Buffer): any {
    const offset = 8; // пропускаем discriminator
    const userWallet = new PublicKey(data.slice(offset, offset + 32));
    const targetAdmin = new PublicKey(data.slice(offset + 32, offset + 64));
    const amount = Number(data.readBigUInt64LE(offset + 64));
    const status = data[offset + 72] as typeof FundingStatus[keyof typeof FundingStatus];
    
    return {
      userWallet,
      targetAdmin,
      amount,
      status,
    };
  }

  /**
   * Создать конфигурацию команды
   */
  createCommandConfig(
    sessionId: number,
    encryptedSessionKey: Uint8Array,
    destination: Destination,
    meta: Uint8Array = new Uint8Array(0)
  ): any {
    return {
      sessionId,
      encryptedSessionKey,
      destination,
      meta,
    };
  }

  /**
   * Сериализовать конфигурацию команды в payload
   */
  serializeCommandConfig(_config: any): Uint8Array {
    // Здесь должна быть реализация Borsh сериализации
    // Для упрощения возвращаем пустой массив
    // В реальной реализации нужно использовать borsh-js
    return new Uint8Array(0);
  }

  /**
   * Создать команду для публикации публичного ключа
   */
  createPublishPubkeyCommand(pubkey: PublicKey): Uint8Array {
    return pubkey.toBuffer();
  }

  /**
   * Создать команду для запроса соединения
   */
  createRequestConnectionCommand(config: CommandConfig): Uint8Array {
    return this.serializeCommandConfig(config);
  }
}
