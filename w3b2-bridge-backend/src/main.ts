import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import morgan from 'morgan';
import dotenv from 'dotenv';
import { SolanaService } from './services/solanaService.js';
import { EncryptionService } from './services/encryptionService.js';
import { DatabaseService } from './services/databaseService.js';
import { apiRoutes } from './routes/api.js';
import { errorHandler } from './middleware/errorHandler.js';

// Загружаем переменные окружения
dotenv.config();

const app = express();
const PORT = process.env.PORT || 3001;

// Middleware
app.use(helmet());
app.use(cors());
app.use(morgan('combined'));
app.use(express.json({ limit: '10mb' }));
app.use(express.urlencoded({ extended: true }));

// Инициализация сервисов
const solanaService = new SolanaService();
const encryptionService = new EncryptionService();
const databaseService = new DatabaseService();

// Передаем сервисы в контекст приложения
app.locals.solanaService = solanaService;
app.locals.encryptionService = encryptionService;
app.locals.databaseService = databaseService;

// Маршруты
app.use('/api', apiRoutes);

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'OK', timestamp: new Date().toISOString() });
});

// Обработка ошибок
app.use(errorHandler);

// Запуск сервера
async function startServer() {
  try {
    // Инициализация сервисов
    await solanaService.initialize();
    await databaseService.initialize();

    // Запуск прослушивания блокчейна
    await solanaService.startBlockchainListener();

    app.listen(PORT, () => {
      console.log(`🚀 W3B2 Backend Server запущен на порту ${PORT}`);
      console.log(`📡 Прослушивание Solana блокчейна...`);
    });
  } catch (error) {
    console.error('❌ Ошибка запуска сервера:', error);
    process.exit(1);
  }
}

startServer();
