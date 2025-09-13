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
import { BlockchainEvent, CommandId, CommandConfig } from '../types/index';
import { serializeCommandConfig } from '../utils/blockchainUtils.js';

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
      process.env.PROGRAM_ID || 'W3B2Bridge111111111111111111111111111111111'
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
      // Проверяем подключение к Solana
      const version = await this.connection.getVersion();
      console.log('✅ Подключение к Solana установлено:', version);

      // Проверяем баланс администратора
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

    // Получаем текущий слот
    this.lastProcessedSlot = await this.connection.getSlot();

    // Запускаем периодическую проверку новых транзакций
    setInterval(async () => {
      await this.checkForNewTransactions();
    }, 5000); // Проверяем каждые 5 секунд
  }

  private async checkForNewTransactions(): Promise<void> {
    try {
      const currentSlot = await this.connection.getSlot();

      if (currentSlot > this.lastProcessedSlot) {
        // Получаем транзакции для нашего программы
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
            await this.processTransaction(sigInfo.signature);
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
          const programId = accountKeys[instruction.programIdIndex];
          if (programId.equals(this.programId)) {
            await this.processProgramInstruction(
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              instruction as any,
              signature,
              transaction
            );
          }
        }
      }
    } catch (error) {
      console.error('❌ Ошибка обработки транзакции:', error);
    }
  }

  private async processProgramInstruction(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    instruction: any,
    signature: string,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    transaction: any
  ): Promise<void> {
    try {
      // Декодируем данные инструкции
      const data = instruction.data;
      const commandId = data.readUInt8(0);

      const event: BlockchainEvent = {
        type: 'command_received',
        data: {
          commandId,
          signature,
          slot: transaction.slot,
          accounts: instruction.accounts || [],
        },
        signature,
        slot: transaction.slot,
        timestamp: Date.now(),
      };

      this.emit('blockchain_event', event);

      // Обрабатываем конкретные команды
      switch (commandId) {
        case CommandId.PUBLISH_PUBKEY:
          await this.handlePublishPubkey(instruction, signature);
          break;
        case CommandId.REQUEST_CONNECTION:
          await this.handleRequestConnection(instruction, signature);
          break;
        case CommandId.CRUD_CREATE:
        case CommandId.CRUD_READ:
        case CommandId.CRUD_UPDATE:
        case CommandId.CRUD_DELETE:
          await this.handleCrudOperation(instruction, signature, commandId);
          break;
        default:
          console.log(`📝 Получена команда ${commandId} от ${signature}`);
      }
    } catch (error) {
      console.error('❌ Ошибка обработки инструкции:', error);
    }
  }

  private async handlePublishPubkey(
    _instruction: any,
    _signature: string
  ): Promise<void> {
    console.log('🔑 Получен публичный ключ от клиента');
  }

  private async handleRequestConnection(
    _instruction: any,
    _signature: string
  ): Promise<void> {
    console.log('🔌 Запрос на установку соединения');
  }

  private async handleCrudOperation(
    _instruction: any,
    _signature: string,
    commandId: number
  ): Promise<void> {
    console.log(`📊 CRUD операция ${commandId}`);
    // Здесь можно добавить логику обработки CRUD операций
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
   */
  async requestFunding(
    userWallet: PublicKey,
    amount: number,
    targetAdmin: PublicKey
  ): Promise<string> {
    try {
      const transaction = new Transaction();

      // Находим PDA для funding request
      const [fundingRequestPDA] = PublicKey.findProgramAddressSync(
        [
          Buffer.from('funding'),
          userWallet.toBuffer(),
          this.adminKeypair.publicKey.toBuffer(), // payer
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
            pubkey: this.adminKeypair.publicKey, // payer
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
          Buffer.from([1]), // request_funding discriminator
          Buffer.alloc(8)
            .fill(0)
            .map((_, i) => (amount >> (i * 8)) & 0xff), // amount as u64
          targetAdmin.toBuffer(), // target_admin as Pubkey (32 bytes)
        ]),
      });

      transaction.add(instruction);

      const signature = await this.connection.sendTransaction(transaction, [
        this.adminKeypair,
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
   */
  async dispatchCommand(
    commandId: number,
    mode: number,
    payload: Uint8Array,
    targetAdmin: PublicKey
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
            pubkey: this.adminKeypair.publicKey, // authority
            isSigner: true,
            isWritable: false,
          },
        ],
        programId: this.programId,
        data,
      });

      transaction.add(instruction);

      const signature = await this.connection.sendTransaction(transaction, [
        this.adminKeypair,
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
   */
  async dispatchCommandConfig(
    commandId: number,
    mode: number,
    config: CommandConfig,
    targetAdmin: PublicKey
  ): Promise<string> {
    try {
      const serializedConfig = serializeCommandConfig(config);
      return await this.dispatchCommand(
        commandId,
        mode,
        serializedConfig,
        targetAdmin
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
}
