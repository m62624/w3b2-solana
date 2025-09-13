# Bridge Client

TypeScript клиент для работы с W3B2 Bridge программой на Solana.

## Установка

```bash
npm install @solana/web3.js @coral-xyz/anchor
```

## Основное использование

### Инициализация

```typescript
import { Connection } from '@solana/web3.js';
import { BridgeClient } from './bridgeClient';

const connection = new Connection('https://api.devnet.solana.com');
const client = new BridgeClient(connection);
```

### Регистрация админа

```typescript
import { Keypair } from '@solana/web3.js';

const payer = Keypair.generate();
const authority = Keypair.generate();
const coSigner = Keypair.generate();

const signature = await client.registerAdmin({
  payer: payer.publicKey,
  authority: authority.publicKey,
  coSigner: coSigner.publicKey,
  initialBalance: 1000000000, // 1 SOL в lamports
}, [payer, authority, coSigner]);
```

### Регистрация пользователя

```typescript
const userWallet = Keypair.generate();
const coSigner = Keypair.generate();

const signature = await client.registerUser({
  payer: payer.publicKey,
  userWallet: userWallet.publicKey,
  coSigner: coSigner.publicKey,
  initialBalance: 500000000, // 0.5 SOL в lamports
}, [payer, userWallet, coSigner]);
```

### Запрос финансирования

```typescript
const [userPDA] = await client.getUserAccountPDA(coSigner.publicKey);
const targetAdmin = Keypair.generate().publicKey;

const signature = await client.requestFunding({
  payer: payer.publicKey,
  userAccount: userPDA,
  amount: 200000000, // 0.2 SOL в lamports
  targetAdmin,
}, [payer]);
```

### Одобрение финансирования

```typescript
const [fundingPDA] = await client.getFundingRequestPDA(userPDA, payer.publicKey);
const adminAuthority = Keypair.generate();

const signature = await client.approveFunding({
  adminAuthority: adminAuthority.publicKey,
  fundingRequest: fundingPDA,
  userWallet: userWallet.publicKey,
}, [adminAuthority]);
```

### Отправка команд

```typescript
import { CommandMode, CMD_PUBLISH_PUBKEY } from '../types/bridge';

// Публикация публичного ключа
const pubkeyCommand = client.createPublishPubkeyCommand(targetPubkey);

const signature = await client.dispatchCommandUser({
  authority: userWallet.publicKey,
  targetPubkey,
  commandId: CMD_PUBLISH_PUBKEY,
  mode: CommandMode.OneWay,
  payload: pubkeyCommand,
}, [userWallet]);
```

### Запрос соединения

```typescript
import { CMD_REQUEST_CONNECTION } from '../types/bridge';

const destination = {
  type: 'url' as const,
  url: 'https://example.com/bridge-endpoint'
};

const config = client.createCommandConfig(
  12345, // sessionId
  encryptedSessionKey, // 80 bytes
  destination,
  new Uint8Array(0) // meta
);

const payload = client.createRequestConnectionCommand(config);

const signature = await client.dispatchCommandUser({
  authority: userWallet.publicKey,
  targetPubkey,
  commandId: CMD_REQUEST_CONNECTION,
  mode: CommandMode.RequestResponse,
  payload,
}, [userWallet]);
```

## Получение данных

### Получение данных аккаунта

```typescript
const [adminPDA] = await client.getAdminAccountPDA(coSigner.publicKey);
const adminAccount = await client.getAdminAccount(adminPDA);

if (adminAccount) {
  console.log('Владелец:', adminAccount.meta.owner.toString());
  console.log('Co-signer:', adminAccount.meta.coSigner.toString());
  console.log('Активен:', adminAccount.meta.active);
}
```

### Получение запроса финансирования

```typescript
const [fundingPDA] = await client.getFundingRequestPDA(userPDA, payer.publicKey);
const fundingRequest = await client.getFundingRequest(fundingPDA);

if (fundingRequest) {
  console.log('Пользователь:', fundingRequest.userWallet.toString());
  console.log('Сумма:', fundingRequest.amount);
  console.log('Статус:', fundingRequest.status);
}
```

## Утилиты

### Использование BridgeUtils

```typescript
import { BridgeUtils } from '../utils/bridgeUtils';

const utils = new BridgeUtils(connection);

// Проверка регистрации
const isAdminRegistered = await utils.isAdminRegistered(coSigner.publicKey);
const isUserRegistered = await utils.isUserRegistered(coSigner.publicKey);

// Получение баланса
const adminBalance = await utils.getAdminBalance(coSigner.publicKey);
const userBalance = await utils.getUserBalance(coSigner.publicKey);

// Создание дестинации
const ipv4Dest = utils.createIPv4Destination('192.168.1.1', 8080);
const urlDest = utils.createURLDestination('https://api.example.com');

// Форматирование
const formattedBalance = utils.formatBalance(adminBalance);
```

### Глобальные утилиты

```typescript
import { bridgeUtils } from '../utils/bridgeUtils';

// Быстрые функции
const isValid = bridgeUtils.isValidAddress('11111111111111111111111111111112');
const formatted = bridgeUtils.formatSOL(1000000000); // "1.0000 SOL"
const lamports = bridgeUtils.toLamports(1.5); // 1500000000
const sol = bridgeUtils.toSOL(1000000000); // 1.0
```

## Обработка ошибок

```typescript
try {
  const signature = await client.registerAdmin(params, signers);
  console.log('Транзакция успешна:', signature);
} catch (error) {
  if (error.message.includes('InsufficientFundsForAdmin')) {
    console.error('Недостаточно средств для создания админского аккаунта');
  } else if (error.message.includes('AlreadyRegistered')) {
    console.error('PDA уже зарегистрирован для этого владельца');
  } else {
    console.error('Неизвестная ошибка:', error);
  }
}
```

## События программы

Клиент поддерживает все события программы:

- `AdminRegistered` - админ зарегистрирован
- `UserRegistered` - пользователь зарегистрирован
- `AdminDeactivated` - админ деактивирован
- `UserDeactivated` - пользователь деактивирован
- `FundingRequested` - запрошено финансирование
- `FundingApproved` - финансирование одобрено
- `CommandEvent` - отправлена команда

## Константы команд

- `CMD_PUBLISH_PUBKEY = 1` - публикация публичного ключа
- `CMD_REQUEST_CONNECTION = 2` - запрос соединения

## Типы данных

Все типы определены в `../types/bridge.ts`:

- `CommandMode` - режим команды (RequestResponse/OneWay)
- `FundingStatus` - статус финансирования (Pending/Approved/Rejected)
- `Destination` - дестинация для команд
- `CommandConfig` - конфигурация команды
- `AccountMeta` - метаданные аккаунта
- `AdminAccount` - админский аккаунт
- `UserAccount` - пользовательский аккаунт
- `FundingRequest` - запрос финансирования

## Примеры

Полные примеры использования доступны в `../examples/bridgeClientExample.ts`.
