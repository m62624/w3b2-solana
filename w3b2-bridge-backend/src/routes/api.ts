import { Router, Request, Response } from 'express';
import { PublicKey, Keypair } from '@solana/web3.js';
import bs58 from 'bs58';
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

    // Создаем keypair из приватного ключа
    const userKeypair = Keypair.fromSecretKey(bs58.decode(userPrivateKey));

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
    const userKeypair = Keypair.fromSecretKey(bs58.decode(userPrivateKey));

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
    const { databaseService, encryptionService } = getServices(req);
    const [dbStats, sessionStats] = await Promise.all([
      databaseService.getStats(),
      Promise.resolve(encryptionService.getSessionStats()),
    ]);

    res.json({
      success: true,
      data: {
        database: dbStats,
        sessions: sessionStats,
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

export { router as apiRoutes };
