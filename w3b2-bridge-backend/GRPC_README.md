# gRPC Сервис для W3B2 Bridge

Этот документ описывает интеграцию gRPC сервиса в W3B2 Bridge Backend.

## Обзор

gRPC сервис предоставляет стриминг событий в реальном времени для мониторинга активности в W3B2 Bridge протоколе.

## Структура

```
w3b2-bridge-backend/
├── proto/
│   └── bridge.proto          # Proto файл с определениями
├── src/
│   ├── types/
│   │   └── bridge.proto.ts   # TypeScript типы
│   ├── services/
│   │   └── grpcService.ts    # gRPC сервер
│   └── examples/
│       └── grpcClient.ts     # Пример клиента
```

## Установка и запуск

### 1. Установка зависимостей

```bash
npm install
```

### 2. Запуск сервера

```bash
npm run dev
```

Сервер запустится на:
- HTTP API: `http://localhost:3001`
- gRPC: `localhost:50051`

### 3. Тестирование gRPC клиента

```bash
npm run grpc:client
```

## API

### BridgeService

#### streamEvents

Стриминг событий в реальном времени.

**Запрос:** `Empty`

**Ответ:** `stream BridgeEvent`

### Типы событий

1. **AdminRegistered** - Регистрация администратора
2. **UserRegistered** - Регистрация пользователя
3. **AdminDeactivated** - Деактивация администратора
4. **UserDeactivated** - Деактивация пользователя
5. **FundingRequested** - Запрос на финансирование
6. **FundingApproved** - Одобрение финансирования
7. **CommandEvent** - Команда между участниками

## Использование в коде

### Отправка события

```typescript
import { GrpcService } from './services/grpcService';

const grpcService = new GrpcService();

// Создание и отправка события
const event = grpcService.createUserRegisteredEvent('user123', 1000);
grpcService.broadcastEvent(event);
```

### Подключение к стриму

```typescript
import { GrpcClient } from './examples/grpcClient';

const client = new GrpcClient('localhost:50051');
await client.connectToEventStream();
```

## Переменные окружения

- `GRPC_PORT` - Порт для gRPC сервера (по умолчанию: 50051)
- `PORT` - Порт для HTTP API (по умолчанию: 3001)

## Мониторинг

Сервис предоставляет методы для мониторинга:

- `getActiveConnectionsCount()` - Количество активных подключений
- `broadcastEvent(event)` - Отправка события всем клиентам

## Безопасность

- gRPC сервер использует insecure credentials (для разработки)
- В продакшене рекомендуется использовать TLS
- Все события логируются в консоль

## Troubleshooting

### Ошибка подключения

Убедитесь, что:
1. Сервер запущен
2. Порт 50051 свободен
3. Proto файл находится в правильной директории

### Ошибки компиляции

```bash
npm run build
```

Проверьте TypeScript ошибки и исправьте их.
