import { Router, Request, Response } from 'express';
import { PublicKey, Keypair } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { ApiResponse, CrudOperation } from '../types/index';

const router = Router();

// Health check endpoint
router.get('/health', (req: Request, res: Response) => {
  res.json({
    success: true,
    data: {
      status: 'OK',
      timestamp: new Date().toISOString(),
    },
    timestamp: new Date().toISOString(),
  } as ApiResponse);
});

// Middleware для извлечения сервисов из контекста приложения
const getServices = (req: Request) => ({
  solanaService: req.app.locals.solanaService,
  encryptionService: req.app.locals.encryptionService,
  databaseService: req.app.locals.databaseService,
  grpcService: req.app.locals.grpcService,
  webSocketService: req.app.locals.webSocketService,
});

// Регистрация пользователя
router.post('/register-user', async (req: Request, res: Response) => {
  try {
    const { publicKey } = req.body;

    if (!publicKey) {
      return res.status(400).json({
        success: false,
        error: 'Публичный ключ обязателен',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { databaseService } = getServices(req);
    const userPublicKey = new PublicKey(publicKey);

    // Проверяем, не зарегистрирован ли уже пользователь
    const existingUser = await databaseService.getUser(userPublicKey);
    if (existingUser) {
      return res.status(409).json({
        success: false,
        error: 'Пользователь уже зарегистрирован',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    // Создаем нового пользователя
    const userAccount = {
      public_key: userPublicKey,
      is_registered: true,
      created_at: Date.now(),
      last_activity: Date.now(),
    };

    await databaseService.createUser(userAccount);

    res.json({
      success: true,
      data: {
        publicKey: userPublicKey.toBase58(),
        serverPublicKey:
          databaseService.encryptionService?.getServerPublicKeyBase58(),
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка регистрации пользователя:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Запрос на финансирование
router.post('/request-funding', async (req: Request, res: Response) => {
  try {
    const { userWallet, amount, targetAdmin, userPrivateKey } = req.body;

    if (!userWallet || !amount || !targetAdmin || !userPrivateKey) {
      return res.status(400).json({
        success: false,
        error: 'Все поля обязательны, включая приватный ключ пользователя',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { databaseService, solanaService } = getServices(req);

    const userWalletPubkey = new PublicKey(userWallet);
    const targetAdminPubkey = new PublicKey(targetAdmin);
    const amountNum = parseInt(amount);

    // Создаем keypair из приватного ключа (base64)
    const userKeypair = Keypair.fromSecretKey(
      Buffer.from(userPrivateKey, 'base64')
    );

    // Проверяем баланс пользователя
    const balance = await solanaService.getBalance(userKeypair.publicKey);
    const minBalance = 0.001; // Минимальный баланс для комиссии

    if (balance < minBalance) {
      return res.status(400).json({
        success: false,
        error: `Недостаточно средств. Текущий баланс: ${balance} SOL. Минимально требуется: ${minBalance} SOL для комиссии. Попробуйте получить airdrop.`,
        data: { balance, minBalance },
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    // Создаем запрос в blockchain
    const signature = await solanaService.requestFunding(
      userWalletPubkey,
      amountNum,
      targetAdminPubkey,
      userKeypair
    );

    // Сохраняем в базе данных
    const fundingRequest = {
      user_wallet: userWalletPubkey,
      amount: amountNum,
      status: 0, // Pending
      target_admin: targetAdminPubkey,
      created_at: Date.now(),
    };

    const requestId =
      await databaseService.createFundingRequest(fundingRequest);

    res.json({
      success: true,
      data: {
        requestId,
        signature,
        message: 'Запрос на финансирование создан в блокчейне',
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка запроса финансирования:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Одобрение финансирования (только для администраторов)
router.post('/approve-funding', async (req: Request, res: Response) => {
  try {
    const { requestId } = req.body;

    if (!requestId) {
      return res.status(400).json({
        success: false,
        error: 'ID запроса обязателен',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { databaseService, solanaService } = getServices(req);

    // Получаем запрос
    const fundingRequest = await databaseService.getFundingRequest(requestId);
    if (!fundingRequest) {
      return res.status(404).json({
        success: false,
        error: 'Запрос не найден',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    if (fundingRequest.status !== 0) {
      // Не Pending
      return res.status(400).json({
        success: false,
        error: 'Запрос уже обработан',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    // Находим PDA для funding request
    const [fundingRequestPDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('funding'),
        fundingRequest.user_wallet.toBuffer(),
        solanaService.getAdminPublicKey().toBuffer(),
      ],
      solanaService.getProgramId()
    );

    // Одобряем в blockchain
    const signature = await solanaService.approveFunding(
      fundingRequestPDA,
      fundingRequest.user_wallet
    );

    // Обновляем статус в базе данных
    await databaseService.updateFundingRequest(requestId, { status: 1 }); // Approved

    res.json({
      success: true,
      data: {
        requestId,
        status: 'approved',
        signature,
        message: 'Финансирование одобрено в блокчейне',
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка одобрения финансирования:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// CRUD операции
router.post('/crud', async (req: Request, res: Response) => {
  try {
    const { operation, owner } = req.body;

    if (!operation || !owner) {
      return res.status(400).json({
        success: false,
        error: 'Операция и владелец обязательны',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { databaseService } = getServices(req);
    const ownerPublicKey = new PublicKey(owner);

    const result = await databaseService.handleCrudOperation(
      operation as CrudOperation,
      ownerPublicKey
    );

    res.json({
      success: true,
      data: result,
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка CRUD операции:', error);
    res.status(500).json({
      success: false,
      error:
        error instanceof Error ? error.message : 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Отправка команды в blockchain
router.post('/dispatch-command', async (req: Request, res: Response) => {
  try {
    const { commandId, mode, config, targetAdmin, userPrivateKey } = req.body;

    if (!commandId || !mode || !config || !targetAdmin || !userPrivateKey) {
      return res.status(400).json({
        success: false,
        error: 'Все поля обязательны, включая приватный ключ пользователя',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { solanaService } = getServices(req);

    const targetAdminPubkey = new PublicKey(targetAdmin);
    const userKeypair = Keypair.fromSecretKey(
      Buffer.from(userPrivateKey, 'base64')
    );

    // Отправляем команду в blockchain
    const signature = await solanaService.dispatchCommandConfig(
      parseInt(commandId),
      parseInt(mode),
      config,
      targetAdminPubkey,
      userKeypair
    );

    res.json({
      success: true,
      data: {
        signature,
        commandId,
        mode,
        message: 'Команда отправлена в блокчейн',
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка отправки команды:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Создание сессии
router.post('/session/create', async (req: Request, res: Response) => {
  try {
    const { clientPublicKey } = req.body;

    if (!clientPublicKey) {
      return res.status(400).json({
        success: false,
        error: 'Публичный ключ клиента обязателен',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { encryptionService } = getServices(req);
    const clientPubKey = new PublicKey(clientPublicKey);

    const session = encryptionService.createSession(clientPubKey);

    res.json({
      success: true,
      data: {
        sessionId: session.sessionId,
        serverPublicKey: session.serverPublicKey.toBase58(),
        expiresAt: session.expiresAt,
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка создания сессии:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Закрытие сессии
router.post('/session/close', async (req: Request, res: Response) => {
  try {
    const { sessionId } = req.body;

    if (!sessionId) {
      return res.status(400).json({
        success: false,
        error: 'ID сессии обязателен',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { encryptionService } = getServices(req);
    const success = encryptionService.closeSession(parseInt(sessionId));

    res.json({
      success,
      data: { sessionId, closed: success },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка закрытия сессии:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение статистики
router.get('/stats', async (req: Request, res: Response) => {
  try {
    const {
      databaseService,
      encryptionService,
      grpcService,
      webSocketService,
    } = getServices(req);
    const [dbStats, sessionStats] = await Promise.all([
      databaseService.getStats(),
      Promise.resolve(encryptionService.getSessionStats()),
    ]);

    res.json({
      success: true,
      data: {
        database: dbStats,
        sessions: sessionStats,
        connections: {
          grpc: {
            connected: grpcService.isConnectorConnected(),
          },
          websocket: webSocketService.getConnectionStats(),
        },
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения статистики:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение всех запросов на финансирование
router.get('/funding-requests', async (req: Request, res: Response) => {
  try {
    const { databaseService } = getServices(req);
    const requests = await databaseService.getAllFundingRequests();

    res.json({
      success: true,
      data: requests,
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения запросов:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Регистрация администратора
router.post('/register-admin', async (req: Request, res: Response) => {
  try {
    const { authority, coSigner, fundingAmount } = req.body;

    if (!authority || !coSigner || !fundingAmount) {
      return res.status(400).json({
        success: false,
        error: 'Все поля обязательны: authority, coSigner, fundingAmount',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { solanaService, databaseService } = getServices(req);

    const authorityPubkey = new PublicKey(authority);
    const coSignerPubkey = new PublicKey(coSigner);
    const fundingAmountNum = parseInt(fundingAmount);

    // Регистрируем администратора в блокчейне
    const signature = await solanaService.registerAdmin(fundingAmountNum);

    // Сохраняем в базе данных
    const adminAccount = {
      public_key: authorityPubkey,
      co_signer: coSignerPubkey,
      is_registered: true,
      funding_amount: fundingAmountNum,
      created_at: Date.now(),
      last_activity: Date.now(),
    };

    await databaseService.createAdmin(adminAccount);

    res.json({
      success: true,
      data: {
        signature,
        adminPublicKey: authorityPubkey.toBase58(),
        coSignerPublicKey: coSignerPubkey.toBase58(),
        fundingAmount: fundingAmountNum,
        message: 'Администратор зарегистрирован в блокчейне',
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка регистрации администратора:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение airdrop для тестирования
router.post('/airdrop', async (req: Request, res: Response) => {
  try {
    const { publicKey } = req.body;

    if (!publicKey) {
      return res.status(400).json({
        success: false,
        error: 'Публичный ключ обязателен',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { solanaService } = getServices(req);
    const userPublicKey = new PublicKey(publicKey);

    // Получаем airdrop (1 SOL для тестирования)
    const signature = await solanaService.requestAirdrop(userPublicKey, 1);

    res.json({
      success: true,
      data: {
        signature,
        message: 'Airdrop получен успешно',
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения airdrop:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение баланса пользователя
router.get('/balance/:publicKey', async (req: Request, res: Response) => {
  try {
    const { publicKey } = req.params;

    if (!publicKey) {
      return res.status(400).json({
        success: false,
        error: 'Публичный ключ обязателен',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { solanaService } = getServices(req);
    const userPublicKey = new PublicKey(publicKey);
    const balance = await solanaService.getBalance(userPublicKey);

    res.json({
      success: true,
      data: {
        publicKey: publicKey,
        balance: balance,
        balanceLamports: balance * 1000000000, // Конвертируем в lamports
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения баланса:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение всех администраторов
router.get('/admins', async (req: Request, res: Response) => {
  try {
    const { databaseService } = getServices(req);
    const admins = await databaseService.getAllAdmins();

    res.json({
      success: true,
      data: admins,
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения администраторов:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение информации о пользователе
router.get('/user/:publicKey', async (req: Request, res: Response) => {
  try {
    const { publicKey } = req.params;

    if (!publicKey) {
      return res.status(400).json({
        success: false,
        error: 'Публичный ключ обязателен',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    const { databaseService } = getServices(req);
    const userPublicKey = new PublicKey(publicKey);
    const user = await databaseService.getUser(userPublicKey);

    if (!user) {
      return res.status(404).json({
        success: false,
        error: 'Пользователь не найден',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    res.json({
      success: true,
      data: user,
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения пользователя:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение событий
router.get('/events', async (req: Request, res: Response) => {
  try {
    const { limit = 100, offset = 0 } = req.query;
    const { databaseService } = getServices(req);

    const events = await databaseService.getEvents(
      parseInt(limit as string),
      parseInt(offset as string)
    );

    res.json({
      success: true,
      data: events,
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения событий:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение событий по типу
router.get('/events/type/:eventType', async (req: Request, res: Response) => {
  try {
    const { eventType } = req.params;
    const { limit = 100 } = req.query;
    const { databaseService } = getServices(req);

    const events = await databaseService.getEventsByType(
      eventType,
      parseInt(limit as string)
    );

    res.json({
      success: true,
      data: events,
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения событий по типу:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение конкретного события
router.get('/events/:eventId', async (req: Request, res: Response) => {
  try {
    const { eventId } = req.params;
    const { databaseService } = getServices(req);

    const event = await databaseService.getEvent(eventId);

    if (!event) {
      return res.status(404).json({
        success: false,
        error: 'Событие не найдено',
        timestamp: new Date().toISOString(),
      } as ApiResponse);
    }

    res.json({
      success: true,
      data: event,
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения события:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Получение статистики событий
router.get('/events/stats', async (req: Request, res: Response) => {
  try {
    const { databaseService, grpcService } = getServices(req);

    const [eventStats, grpcStats] = await Promise.all([
      databaseService.getEventStats(),
      Promise.resolve(grpcService.getEventStats()),
    ]);

    res.json({
      success: true,
      data: {
        events: eventStats,
        grpc: grpcStats,
      },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка получения статистики событий:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

// Очистка старых событий
router.post('/events/cleanup', async (req: Request, res: Response) => {
  try {
    const { maxAge = 7 * 24 * 60 * 60 * 1000 } = req.body; // 7 дней по умолчанию
    const { databaseService } = getServices(req);

    await databaseService.cleanupOldEvents(maxAge);

    res.json({
      success: true,
      data: { message: 'Старые события очищены' },
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  } catch (error) {
    console.error('❌ Ошибка очистки событий:', error);
    res.status(500).json({
      success: false,
      error: 'Внутренняя ошибка сервера',
      timestamp: new Date().toISOString(),
    } as ApiResponse);
  }
});

export { router as apiRoutes };
