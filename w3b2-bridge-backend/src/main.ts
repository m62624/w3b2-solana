import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import morgan from 'morgan';
import dotenv from 'dotenv';
import { SolanaService } from './services/solanaService';
import { EncryptionService } from './services/encryptionService';
import { DatabaseService } from './services/databaseService';
import { GrpcService } from './services/grpcService';
import { apiRoutes } from './routes/api';
import { errorHandler, notFoundHandler } from './middleware/errorHandler';

// Загружаем переменные окружения
dotenv.config();

const app = express();
const PORT = process.env.PORT || 3001;
const GRPC_PORT = process.env.GRPC_PORT || '50051';

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
const grpcService = new GrpcService();

// Передаем сервисы в контекст приложения
app.locals.solanaService = solanaService;
app.locals.encryptionService = encryptionService;
app.locals.databaseService = databaseService;
app.locals.grpcService = grpcService;

// Маршруты
app.use('/api', apiRoutes);

// Обработка 404 ошибок
app.use(notFoundHandler);

// Обработка ошибок
app.use(errorHandler);

// Запуск сервера
async function startServer() {
  try {
    // Инициализация сервисов
    await solanaService.initialize();
    await databaseService.initialize();

    // Запуск gRPC сервера
    await grpcService.start(GRPC_PORT);

    // Запуск прослушивания блокчейна
    await solanaService.startBlockchainListener();

    app.listen(PORT, () => {
      console.log(`🚀 W3B2 Backend Server запущен на порту ${PORT}`);
      console.log(`🔌 gRPC сервер запущен на порту ${GRPC_PORT}`);
      console.log(`📡 Прослушивание Solana блокчейна...`);
    });
  } catch (error) {
    console.error('❌ Ошибка запуска сервера:', error);
    process.exit(1);
  }
}

startServer();

// Обработка graceful shutdown
process.on('SIGINT', async () => {
  console.log('\n🛑 Получен сигнал SIGINT, завершение работы...');
  try {
    await grpcService.stop();
    console.log('✅ gRPC сервер остановлен');
    process.exit(0);
  } catch (error) {
    console.error('❌ Ошибка при остановке gRPC сервера:', error);
    process.exit(1);
  }
});

process.on('SIGTERM', async () => {
  console.log('\n🛑 Получен сигнал SIGTERM, завершение работы...');
  try {
    await grpcService.stop();
    console.log('✅ gRPC сервер остановлен');
    process.exit(0);
  } catch (error) {
    console.error('❌ Ошибка при остановке gRPC сервера:', error);
    process.exit(1);
  }
});
