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
      data: this.encodeRegisterAdminData(params.fundingAmount),
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
  private encodeRegisterAdminData(fundingAmount: number): Buffer {
    const data = Buffer.alloc(8 + 8); // discriminator + u64
    // Дискриминатор для register_admin: [232, 95, 10, 59, 200, 222, 147, 38]
    data.set([232, 95, 10, 59, 200, 222, 147, 38], 0);
    data.writeBigUInt64LE(BigInt(fundingAmount), 8);
    return data;
  }

  private encodeRegisterUserData(initialBalance: number): Buffer {
    const data = Buffer.alloc(8 + 8); // discriminator + u64
    // Дискриминатор для register_user: [2, 241, 150, 223, 99, 214, 116, 97]
    data.set([2, 241, 150, 223, 99, 214, 116, 97], 0);
    data.writeBigUInt64LE(BigInt(initialBalance), 8);
    return data;
  }

  private encodeDeactivateAdminData(): Buffer {
    const data = Buffer.alloc(8);
    // Дискриминатор для deactivate_admin: [186, 94, 190, 12, 33, 207, 53, 6]
    data.set([186, 94, 190, 12, 33, 207, 53, 6], 0);
    return data;
  }

  private encodeDeactivateUserData(): Buffer {
    const data = Buffer.alloc(8);
    // Дискриминатор для deactivate_user: [170, 53, 163, 46, 104, 99, 39, 15]
    data.set([170, 53, 163, 46, 104, 99, 39, 15], 0);
    return data;
  }

  private encodeRequestFundingData(amount: number, targetAdmin: PublicKey): Buffer {
    const data = Buffer.alloc(8 + 8 + 32); // discriminator + u64 + pubkey
    // Дискриминатор для request_funding: [181, 251, 230, 32, 73, 41, 179, 115]
    data.set([181, 251, 230, 32, 73, 41, 179, 115], 0);
    data.writeBigUInt64LE(BigInt(amount), 8);
    data.set(targetAdmin.toBuffer(), 16);
    return data;
  }

  private encodeApproveFundingData(): Buffer {
    const data = Buffer.alloc(8);
    // Дискриминатор для approve_funding: [141, 177, 12, 63, 22, 7, 248, 100]
    data.set([141, 177, 12, 63, 22, 7, 248, 100], 0);
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

}
