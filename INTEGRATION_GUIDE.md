# Интеграция gRPC коннектора с фронтендом через бэкенд

## Обзор

Данная интеграция обеспечивает передачу событий из gRPC коннектора во фронтенд через бэкенд с использованием WebSocket соединения.

## Архитектура

```
[gRPC Connector] --gRPC--> [Backend] --WebSocket--> [Frontend]
     ↓                           ↓                      ↓
[Solana Blockchain]        [Event Processing]    [Real-time UI]
```

## Компоненты

### 1. gRPC Connector (w3b2-connector)
- Слушает события Solana блокчейна
- Предоставляет gRPC сервер на порту 50051
- Стримит события через `streamEvents`

### 2. Backend (w3b2-bridge-backend)
- Подключается к gRPC коннектору как клиент
- Получает события через gRPC
- Пересылает события через WebSocket на порту 3001
- Предоставляет REST API

### 3. Frontend (w3b2-bridge-frontend)
- Подключается к WebSocket серверу
- Получает real-time события
- Отображает события в UI

## Настройка и запуск

### 1. Запуск gRPC коннектора

```bash
cd w3b2-connector
cargo run
```

Коннектор запустится на порту 50051.

### 2. Запуск бэкенда

```bash
cd w3b2-bridge-backend
npm install
npm run dev
```

Бэкенд запустится на порту 3001 с WebSocket поддержкой.

### 3. Запуск фронтенда

```bash
cd w3b2-bridge-frontend
npm install
npm start
```

Фронтенд запустится на порту 3000.

## Переменные окружения

### Backend (.env)
```env
PORT=3001
GRPC_PORT=50052
CONNECTOR_GRPC_URL=localhost:50051
FRONTEND_URL=http://localhost:3000
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr
```

### Frontend (.env)
```env
REACT_APP_API_URL=http://localhost:3001/api
REACT_APP_WS_URL=http://localhost:3001
REACT_APP_SOLANA_RPC_URL=https://api.devnet.solana.com
REACT_APP_PROGRAM_ID=3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr
```

## Поток данных

1. **Событие в блокчейне** → gRPC коннектор получает событие
2. **gRPC стрим** → Коннектор отправляет событие через gRPC
3. **Backend обработка** → Бэкенд получает событие и обрабатывает
4. **WebSocket broadcast** → Бэкенд отправляет событие всем подключенным клиентам
5. **Frontend обновление** → Фронтенд получает событие и обновляет UI

## Типы событий

- `AdminRegistered` - Регистрация администратора
- `UserRegistered` - Регистрация пользователя
- `FundingRequested` - Запрос на финансирование
- `FundingApproved` - Одобрение финансирования
- `CommandEvent` - Отправка команды
- `AdminDeactivated` - Деактивация администратора
- `UserDeactivated` - Деактивация пользователя

## WebSocket API

### События от сервера
- `connected` - Подключение установлено
- `blockchain_event` - Событие блокчейна
- `notification` - Уведомление
- `status` - Статус сервера

### События к серверу
- `subscribe_events` - Подписка на события
- `unsubscribe_events` - Отписка от событий
- `get_status` - Запрос статуса

## Отладка

### Проверка соединений
1. **gRPC коннектор**: Логи в консоли коннектора
2. **Backend**: Логи в консоли бэкенда
3. **Frontend**: Откройте DevTools → Network → WS

### Тестирование
1. Создайте транзакцию в Solana
2. Проверьте логи коннектора
3. Проверьте логи бэкенда
4. Проверьте WebSocket соединение во фронтенде

## Устранение неполадок

### gRPC коннектор не подключается
- Проверьте, что коннектор запущен на порту 50051
- Проверьте логи коннектора

### Backend не получает события
- Проверьте переменную `CONNECTOR_GRPC_URL`
- Проверьте логи бэкенда

### Frontend не получает события
- Проверьте WebSocket соединение в DevTools
- Проверьте переменную `REACT_APP_WS_URL`
- Проверьте CORS настройки

## Мониторинг

### Статистика соединений
- GET `/api/stats` - Получить статистику всех соединений

### WebSocket статус
- Во фронтенде отображается статус WebSocket соединения
- В боковой панели показывается статус всех соединений

## Безопасность

- WebSocket соединения защищены CORS
- gRPC соединения используют insecure credentials (для разработки)
- В продакшене используйте TLS для gRPC

## Производительность

- WebSocket соединения автоматически переподключаются
- События кэшируются в памяти (последние 100)
- Автоматическая очистка старых событий
