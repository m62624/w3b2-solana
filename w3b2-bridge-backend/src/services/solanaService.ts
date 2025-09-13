import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  SystemProgram,
  LAMPORTS_PER_SOL,
  TransactionInstruction,
} from '@solana/web3.js';
import { EventEmitter } from 'events';
import bs58 from 'bs58';
import { BlockchainEvent, CommandConfig } from '../types/index';
import { serializeCommandConfig } from '../utils/blockchainUtils';

export class SolanaService extends EventEmitter {
  private connection: Connection;
  private programId: PublicKey;
  private adminKeypair: Keypair;
  private isListening: boolean = false;
  private lastProcessedSlot: number = 0;

  constructor() {
    super();
    this.connection = new Connection(
      process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com',
      'confirmed'
    );
    this.programId = new PublicKey(
      process.env.PROGRAM_ID || '3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr'
    );

    // Генерируем или загружаем ключи администратора
    const adminPrivateKey = process.env.ADMIN_PRIVATE_KEY;
    if (adminPrivateKey) {
      this.adminKeypair = Keypair.fromSecretKey(bs58.decode(adminPrivateKey));
    } else {
      this.adminKeypair = Keypair.generate();
      console.log(
        '🔑 Новый ключ администратора:',
        bs58.encode(this.adminKeypair.secretKey)
      );
    }
  }

  async initialize(): Promise<void> {
    try {
      const version = await this.connection.getVersion();
      console.log('✅ Подключение к Solana установлено:', version);

      const balance = await this.connection.getBalance(
        this.adminKeypair.publicKey
      );
      console.log(
        `💰 Баланс администратора: ${balance / LAMPORTS_PER_SOL} SOL`
      );

      if (balance < 0.1 * LAMPORTS_PER_SOL) {
        console.warn(
          '⚠️ Низкий баланс администратора. Рекомендуется пополнить.'
        );
      }
    } catch (error) {
      console.error('❌ Ошибка инициализации Solana сервиса:', error);
      throw error;
    }
  }

  async startBlockchainListener(): Promise<void> {
    if (this.isListening) {
      console.log('📡 Прослушивание уже запущено');
      return;
    }

    this.isListening = true;
    console.log('📡 Запуск прослушивания блокчейна...');

    this.lastProcessedSlot = await this.connection.getSlot();

    setInterval(async () => {
      await this.checkForNewTransactions();
    }, 5000);
  }

  private async checkForNewTransactions(): Promise<void> {
    try {
      const currentSlot = await this.connection.getSlot();

      if (currentSlot > this.lastProcessedSlot) {
        const signatures = await this.connection.getSignaturesForAddress(
          this.programId,
          {
            before: undefined,
            until: undefined,
            limit: 100,
          }
        );

        for (const sigInfo of signatures) {
          if (sigInfo.slot > this.lastProcessedSlot) {
            try {
              await this.processTransaction(sigInfo.signature);
            } catch (error) {
              console.error(
                `❌ Ошибка обработки транзакции ${sigInfo.signature}:`,
                error
              );
              // Продолжаем обработку других транзакций даже если одна не удалась
            }
          }
        }

        this.lastProcessedSlot = currentSlot;
      }
    } catch (error) {
      console.error('❌ Ошибка при проверке транзакций:', error);
    }
  }

  private async processTransaction(signature: string): Promise<void> {
    try {
      const transaction = await this.connection.getTransaction(signature, {
        commitment: 'confirmed',
        maxSupportedTransactionVersion: 0,
      });

      if (!transaction) return;

      // Анализируем инструкции в транзакции
      const message = transaction.transaction.message;
      const instructions =
        'instructions' in message ? message.instructions : [];

      for (const instruction of instructions) {
        if (instruction.programIdIndex !== undefined) {
          const accountKeys =
            'getAccountKeys' in message
              ? message.getAccountKeys()
              : // eslint-disable-next-line @typescript-eslint/no-explicit-any
                (message as any).accountKeys;

          // Проверяем, что accountKeys существует и programIdIndex в пределах массива
          if (
            !accountKeys ||
            instruction.programIdIndex >= accountKeys.length
          ) {
            console.warn(
              `⚠️ Некорректный programIdIndex ${instruction.programIdIndex} для транзакции ${signature}`
            );
            continue;
          }

          const programId = accountKeys[instruction.programIdIndex];

          // Проверяем, что programId существует
          if (!programId) {
            console.warn(
              `⚠️ Некорректный programId для транзакции ${signature}`
            );
            continue;
          }

          // Преобразуем programId в PublicKey если это строка
          let programIdPubkey: PublicKey;
          try {
            programIdPubkey =
              typeof programId === 'string'
                ? new PublicKey(programId)
                : programId;
          } catch {
            console.warn(
              `⚠️ Некорректный формат programId для транзакции ${signature}:`,
              programId
            );
            continue;
          }

          if (programIdPubkey.equals(this.programId)) {
            console.log(`✅ Найдена транзакция W3B2 Bridge: ${signature}`);
            await this.processProgramInstruction(
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              instruction as any,
              signature,
              transaction
            );
          } else {
            console.log(
              `ℹ️ Транзакция ${signature} не относится к W3B2 Bridge. ProgramId: ${programIdPubkey.toString()}, ожидаемый: ${this.programId.toString()}`
            );
          }
        }
      }
    } catch (error) {
      console.error('❌ Ошибка обработки транзакции:', error);
    }
  }

  private async processProgramInstruction(
    instruction: { data: Buffer; accounts?: number[] },
    signature: string,
    transaction: { slot: number }
  ): Promise<void> {
    try {
      // Декодируем данные инструкции
      const data = instruction.data;
      const discriminator = data.readUInt8(0);

      // Обрабатываем события W3B2 Bridge Protocol
      switch (discriminator) {
        case 0: // register_admin
          await this.handleAdminRegistration(
            instruction,
            signature,
            transaction
          );
          break;
        case 1: // request_funding
          await this.handleFundingRequest(instruction, signature, transaction);
          break;
        case 2: // approve_funding
          await this.handleFundingApproval(instruction, signature, transaction);
          break;
        case 3: // dispatch_command
          await this.handleCommandDispatch(instruction, signature, transaction);
          break;
        default:
          console.log(
            `📝 Получена неизвестная инструкция ${discriminator} от ${signature}`
          );
      }
    } catch (error) {
      console.error('❌ Ошибка обработки инструкции:', error);
    }
  }

  private async handleAdminRegistration(
    instruction: { data: Buffer; accounts?: number[] },
    signature: string,
    transaction: { slot: number }
  ): Promise<void> {
    console.log('👑 Администратор зарегистрирован:', signature);

    const event: BlockchainEvent = {
      type: 'admin_registered',
      data: {
        signature,
        slot: transaction.slot,
        accounts: instruction.accounts || [],
      },
      signature,
      slot: transaction.slot,
      timestamp: Date.now(),
    };

    this.emit('blockchain_event', event);
  }

  private async handleFundingRequest(
    instruction: { data: Buffer; accounts?: number[] },
    signature: string,
    transaction: { slot: number }
  ): Promise<void> {
    console.log('💰 Получен запрос на финансирование:', signature);

    // Декодируем данные запроса
    const data = instruction.data;
    const amount = data.readBigUInt64LE(1); // amount (8 bytes)
    const targetAdmin = data.slice(9, 41); // target_admin (32 bytes)

    const event: BlockchainEvent = {
      type: 'funding_requested',
      data: {
        signature,
        slot: transaction.slot,
        amount: Number(amount),
        targetAdmin: Buffer.from(targetAdmin).toString('base64'),
        accounts: instruction.accounts || [],
      },
      signature,
      slot: transaction.slot,
      timestamp: Date.now(),
    };

    this.emit('blockchain_event', event);
  }

  private async handleFundingApproval(
    instruction: { data: Buffer; accounts?: number[] },
    signature: string,
    transaction: { slot: number }
  ): Promise<void> {
    console.log('✅ Финансирование одобрено:', signature);

    const event: BlockchainEvent = {
      type: 'funding_approved',
      data: {
        signature,
        slot: transaction.slot,
        accounts: instruction.accounts || [],
      },
      signature,
      slot: transaction.slot,
      timestamp: Date.now(),
    };

    this.emit('blockchain_event', event);
  }

  private async handleCommandDispatch(
    instruction: { data: Buffer; accounts?: number[] },
    signature: string,
    transaction: { slot: number }
  ): Promise<void> {
    console.log('📤 Получена команда:', signature);

    // Декодируем данные команды
    const data = instruction.data;
    const commandId = data.readBigUInt64LE(1); // command_id (8 bytes)
    const mode = data.readUInt8(9); // mode (1 byte)
    const payloadLength = data.readUInt32LE(10); // payload length (4 bytes)
    const payload = data.slice(14, 14 + payloadLength); // payload
    const targetAdmin = data.slice(14 + payloadLength, 14 + payloadLength + 32); // target_admin (32 bytes)

    const event: BlockchainEvent = {
      type: 'command_dispatched',
      data: {
        signature,
        slot: transaction.slot,
        commandId: Number(commandId),
        mode,
        payload: Buffer.from(payload).toString('base64'),
        targetAdmin: Buffer.from(targetAdmin).toString('base64'),
        accounts: instruction.accounts || [],
      },
      signature,
      slot: transaction.slot,
      timestamp: Date.now(),
    };

    this.emit('blockchain_event', event);
  }

  // Методы для работы с blockchain инструкциями

  /**
   * Регистрирует администратора в blockchain программе
   */
  async registerAdmin(fundingAmount: number): Promise<string> {
    try {
      const transaction = new Transaction();

      // Находим PDA для admin профиля
      const [adminProfilePDA] = PublicKey.findProgramAddressSync(
        [Buffer.from('admin'), this.adminKeypair.publicKey.toBuffer()],
        this.programId
      );

      // Создаем инструкцию для регистрации администратора
      const instruction = new TransactionInstruction({
        keys: [
          {
            pubkey: adminProfilePDA,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: this.adminKeypair.publicKey,
            isSigner: true,
            isWritable: true,
          },
          {
            pubkey: this.adminKeypair.publicKey, // payer
            isSigner: true,
            isWritable: true,
          },
          {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ],
        programId: this.programId,
        data: Buffer.concat([
          Buffer.from([0]), // register_admin discriminator
          Buffer.alloc(8)
            .fill(0)
            .map((_, i) => (fundingAmount >> (i * 8)) & 0xff), // funding_amount as u64
        ]),
      });

      transaction.add(instruction);

      const signature = await this.connection.sendTransaction(transaction, [
        this.adminKeypair,
      ]);
      await this.connection.confirmTransaction(signature);

      console.log('✅ Администратор зарегистрирован:', signature);
      return signature;
    } catch (error) {
      console.error('❌ Ошибка регистрации администратора:', error);
      throw error;
    }
  }

  /**
   * Создает запрос на финансирование
   * ВАЖНО: Эта функция должна вызываться с фронтенда с подписью пользователя
   */
  async requestFunding(
    userWallet: PublicKey,
    amount: number,
    targetAdmin: PublicKey,
    userKeypair: Keypair
  ): Promise<string> {
    try {
      const transaction = new Transaction();

      // Находим PDA для funding request
      const [fundingRequestPDA] = PublicKey.findProgramAddressSync(
        [
          Buffer.from('funding'),
          userWallet.toBuffer(),
          userKeypair.publicKey.toBuffer(), // payer должен быть пользователь
        ],
        this.programId
      );

      const instruction = new TransactionInstruction({
        keys: [
          {
            pubkey: fundingRequestPDA,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: userKeypair.publicKey, // payer - пользователь
            isSigner: true,
            isWritable: true,
          },
          {
            pubkey: userWallet,
            isSigner: false,
            isWritable: false,
          },
          {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ],
        programId: this.programId,
        data: Buffer.concat([
          Buffer.from([181, 251, 230, 32, 73, 41, 179, 115]), // request_funding discriminator
          Buffer.alloc(8)
            .fill(0)
            .map((_, i) => (amount >> (i * 8)) & 0xff), // amount as u64
          targetAdmin.toBuffer(), // target_admin as Pubkey (32 bytes)
        ]),
      });

      transaction.add(instruction);

      const signature = await this.connection.sendTransaction(transaction, [
        userKeypair, // подписывает пользователь
      ]);
      await this.connection.confirmTransaction(signature);

      console.log('✅ Запрос на финансирование создан:', signature);
      return signature;
    } catch (error) {
      console.error('❌ Ошибка создания запроса на финансирование:', error);
      throw error;
    }
  }

  /**
   * Одобряет запрос на финансирование
   */
  async approveFunding(
    fundingRequestPDA: PublicKey,
    userWallet: PublicKey
  ): Promise<string> {
    try {
      const transaction = new Transaction();

      // Находим PDA для admin профиля
      const [adminProfilePDA] = PublicKey.findProgramAddressSync(
        [Buffer.from('admin'), this.adminKeypair.publicKey.toBuffer()],
        this.programId
      );

      const instruction = new TransactionInstruction({
        keys: [
          {
            pubkey: adminProfilePDA,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: fundingRequestPDA,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: userWallet,
            isSigner: false,
            isWritable: true,
          },
          {
            pubkey: this.adminKeypair.publicKey, // admin_authority
            isSigner: true,
            isWritable: false,
          },
          {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ],
        programId: this.programId,
        data: Buffer.from([2]), // approve_funding discriminator
      });

      transaction.add(instruction);

      const signature = await this.connection.sendTransaction(transaction, [
        this.adminKeypair,
      ]);
      await this.connection.confirmTransaction(signature);

      console.log('✅ Финансирование одобрено:', signature);
      return signature;
    } catch (error) {
      console.error('❌ Ошибка одобрения финансирования:', error);
      throw error;
    }
  }

  /**
   * Отправляет команду в blockchain
   * ВАЖНО: Эта функция должна вызываться с фронтенда с подписью пользователя
   */
  async dispatchCommand(
    commandId: number,
    mode: number,
    payload: Uint8Array,
    targetAdmin: PublicKey,
    userKeypair: Keypair
  ): Promise<string> {
    try {
      const transaction = new Transaction();

      const data = Buffer.concat([
        Buffer.from([3]), // dispatch_command discriminator
        Buffer.alloc(8)
          .fill(0)
          .map((_, i) => (commandId >> (i * 8)) & 0xff), // command_id as u64
        Buffer.from([mode]), // mode as u8
        Buffer.alloc(4)
          .fill(0)
          .map((_, i) => (payload.length >> (i * 8)) & 0xff), // payload length as u32
        Buffer.from(payload), // payload
        targetAdmin.toBuffer(), // target_admin as Pubkey
      ]);

      const instruction = new TransactionInstruction({
        keys: [
          {
            pubkey: userKeypair.publicKey, // authority - пользователь
            isSigner: true,
            isWritable: false,
          },
        ],
        programId: this.programId,
        data,
      });

      transaction.add(instruction);

      const signature = await this.connection.sendTransaction(transaction, [
        userKeypair, // подписывает пользователь
      ]);
      await this.connection.confirmTransaction(signature);

      console.log(`📤 Команда ${commandId} отправлена:`, signature);
      return signature;
    } catch (error) {
      console.error('❌ Ошибка отправки команды:', error);
      throw error;
    }
  }

  /**
   * Отправляет CommandConfig как команду
   * ВАЖНО: Эта функция должна вызываться с фронтенда с подписью пользователя
   */
  async dispatchCommandConfig(
    commandId: number,
    mode: number,
    config: CommandConfig,
    targetAdmin: PublicKey,
    userKeypair: Keypair
  ): Promise<string> {
    try {
      const serializedConfig = serializeCommandConfig(config);
      return await this.dispatchCommand(
        commandId,
        mode,
        serializedConfig,
        targetAdmin,
        userKeypair
      );
    } catch (error) {
      console.error('❌ Ошибка отправки CommandConfig:', error);
      throw error;
    }
  }

  getAdminPublicKey(): PublicKey {
    return this.adminKeypair.publicKey;
  }

  getProgramId(): PublicKey {
    return this.programId;
  }

  stopListening(): void {
    this.isListening = false;
    console.log('🛑 Прослушивание блокчейна остановлено');
  }

  /**
   * Получает баланс аккаунта
   */
  async getBalance(publicKey: PublicKey): Promise<number> {
    try {
      const balance = await this.connection.getBalance(publicKey);
      return balance / LAMPORTS_PER_SOL;
    } catch (error) {
      console.error('❌ Ошибка получения баланса:', error);
      throw error;
    }
  }

  /**
   * Запрашивает airdrop для тестирования
   */
  async requestAirdrop(
    publicKey: PublicKey,
    solAmount: number = 1
  ): Promise<string> {
    try {
      const lamports = solAmount * LAMPORTS_PER_SOL;
      const signature = await this.connection.requestAirdrop(
        publicKey,
        lamports
      );

      // Ждем подтверждения
      await this.connection.confirmTransaction(signature);

      console.log(`💰 Airdrop ${solAmount} SOL получен:`, signature);
      return signature;
    } catch (error) {
      console.error('❌ Ошибка получения airdrop:', error);
      throw error;
    }
  }
}
