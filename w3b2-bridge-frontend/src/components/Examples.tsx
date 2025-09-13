import React, { useState, useEffect } from 'react';
import { 
  Play, 
  CheckCircle, 
  XCircle, 
  Loader2, 
  ExternalLink,
  Shield,
  User,
  CreditCard,
  Send,
  Database
} from 'lucide-react';
import { useWalletContext } from '../contexts/WalletContext';
import { BridgeClient } from '../services/bridgeClient';
import { BridgeUtils } from '../utils/bridgeUtils';
import { CommandMode, CMD_PUBLISH_PUBKEY } from '../types/bridge';
import { Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import toast from 'react-hot-toast';
import ExampleResult from './ExampleResult';
import ExamplesStats from './ExamplesStats';
import CodeBlock from './CodeBlock';
import ProgramInfo from './ProgramInfo';

interface ExampleResultData {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'success' | 'error';
  result?: any;
  error?: string;
  signature?: string;
}

const Examples: React.FC = () => {
  const { connection } = useWalletContext();
  const [client, setClient] = useState<BridgeClient | null>(null);
  const [utils, setUtils] = useState<BridgeUtils | null>(null);
  const [results, setResults] = useState<ExampleResultData[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [selectedExample, setSelectedExample] = useState<string | null>(null);

  useEffect(() => {
    if (connection) {
      const bridgeClient = new BridgeClient(connection);
      const bridgeUtils = new BridgeUtils(connection);
      setClient(bridgeClient);
      setUtils(bridgeUtils);
    }
  }, [connection]);

  const examples = [
    {
      id: 'register-admin',
      name: 'Регистрация админа',
      description: 'Создание нового админского аккаунта с начальным финансированием',
      icon: Shield,
      color: 'bg-blue-500',
      code: `const payer = Keypair.generate();
const authority = Keypair.generate();
const coSigner = Keypair.generate();

const signature = await client.registerAdmin({
  payer: payer.publicKey,
  authority: authority.publicKey,
  coSigner: coSigner.publicKey,
  initialBalance: 1000000000, // 1 SOL
}, [payer, authority, coSigner]);`
    },
    {
      id: 'register-user',
      name: 'Регистрация пользователя',
      description: 'Создание нового пользовательского аккаунта',
      icon: User,
      color: 'bg-green-500',
      code: `const payer = Keypair.generate();
const userWallet = Keypair.generate();
const coSigner = Keypair.generate();

const signature = await client.registerUser({
  payer: payer.publicKey,
  userWallet: userWallet.publicKey,
  coSigner: coSigner.publicKey,
  initialBalance: 500000000, // 0.5 SOL
}, [payer, userWallet, coSigner]);`
    },
    {
      id: 'request-funding',
      name: 'Запрос финансирования',
      description: 'Пользователь запрашивает финансирование у админа',
      icon: CreditCard,
      color: 'bg-yellow-500',
      code: `const [userPDA] = await client.getUserAccountPDA(coSigner.publicKey);
const targetAdmin = Keypair.generate().publicKey;

const signature = await client.requestFunding({
  payer: payer.publicKey,
  userAccount: userPDA,
  amount: 200000000, // 0.2 SOL
  targetAdmin,
}, [payer]);`
    },
    {
      id: 'approve-funding',
      name: 'Одобрение финансирования',
      description: 'Админ одобряет запрос на финансирование',
      icon: CheckCircle,
      color: 'bg-emerald-500',
      code: `const [fundingPDA] = await client.getFundingRequestPDA(userPDA, payer.publicKey);
const adminAuthority = Keypair.generate();

const signature = await client.approveFunding({
  adminAuthority: adminAuthority.publicKey,
  fundingRequest: fundingPDA,
  userWallet: userWallet.publicKey,
}, [adminAuthority]);`
    },
    {
      id: 'dispatch-command',
      name: 'Отправка команды',
      description: 'Отправка команды пользователем или админом',
      icon: Send,
      color: 'bg-purple-500',
      code: `const pubkeyCommand = client.createPublishPubkeyCommand(targetPubkey);

const signature = await client.dispatchCommandUser({
  authority: userWallet.publicKey,
  targetPubkey,
  commandId: CMD_PUBLISH_PUBKEY,
  mode: CommandMode.OneWay,
  payload: pubkeyCommand,
}, [userWallet]);`
    },
    {
      id: 'connection-request',
      name: 'Запрос соединения',
      description: 'Создание команды для запроса соединения с сервисом',
      icon: ExternalLink,
      color: 'bg-indigo-500',
      code: `const destination = {
  type: 'url',
  url: 'https://example.com/bridge-endpoint'
};

const config = client.createCommandConfig(
  12345, // sessionId
  encryptedSessionKey, // 80 bytes
  destination,
  new Uint8Array(0) // meta
);

const payload = client.createRequestConnectionCommand(config);`
    }
  ];

  const runExample = async (exampleId: string) => {
    if (!client || !utils) {
      toast.error('Клиент не инициализирован');
      return;
    }

    setSelectedExample(exampleId);
    setIsRunning(true);

    // Обновляем статус
    setResults(prev => prev.map(r => 
      r.id === exampleId 
        ? { ...r, status: 'running' as const }
        : r
    ));

    try {
      let result: any = null;
      let signature: string | undefined;

      switch (exampleId) {
        case 'register-admin': {
          const payer = Keypair.generate();
          const authority = Keypair.generate();
          const coSigner = Keypair.generate();

          // Получаем SOL для тестирования
          try {
            const airdropSignature = await connection!.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
            await connection!.confirmTransaction(airdropSignature);
          } catch (error) {
            console.warn('Не удалось получить airdrop:', error);
          }

          signature = await client.registerAdmin({
            payer: payer.publicKey,
            authority: authority.publicKey,
            coSigner: coSigner.publicKey,
            initialBalance: 0.1 * LAMPORTS_PER_SOL,
          }, [payer, authority, coSigner]);

          const [adminPDA] = await client.getAdminAccountPDA(coSigner.publicKey);
          const adminAccount = await client.getAdminAccount(adminPDA);
          
          result = {
            signature,
            adminPDA: adminPDA.toString(),
            adminAccount
          };
          break;
        }

        case 'register-user': {
          const payer = Keypair.generate();
          const userWallet = Keypair.generate();
          const coSigner = Keypair.generate();

          try {
            const airdropSignature = await connection!.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
            await connection!.confirmTransaction(airdropSignature);
          } catch (error) {
            console.warn('Не удалось получить airdrop:', error);
          }

          signature = await client.registerUser({
            payer: payer.publicKey,
            userWallet: userWallet.publicKey,
            coSigner: coSigner.publicKey,
            initialBalance: 0.05 * LAMPORTS_PER_SOL,
          }, [payer, userWallet, coSigner]);

          const [userPDA] = await client.getUserAccountPDA(coSigner.publicKey);
          const userAccount = await client.getUserAccount(userPDA);
          
          result = {
            signature,
            userPDA: userPDA.toString(),
            userAccount
          };
          break;
        }

        case 'request-funding': {
          const payer = Keypair.generate();
          const userWallet = Keypair.generate();
          const coSigner = Keypair.generate();
          const targetAdmin = Keypair.generate().publicKey;

          try {
            const airdropSignature = await connection!.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
            await connection!.confirmTransaction(airdropSignature);
          } catch (error) {
            console.warn('Не удалось получить airdrop:', error);
          }

          // Сначала регистрируем пользователя
          await client.registerUser({
            payer: payer.publicKey,
            userWallet: userWallet.publicKey,
            coSigner: coSigner.publicKey,
            initialBalance: 0,
          }, [payer, userWallet, coSigner]);

          const [userPDA] = await client.getUserAccountPDA(coSigner.publicKey);

          signature = await client.requestFunding({
            payer: payer.publicKey,
            userAccount: userPDA,
            amount: 0.1 * LAMPORTS_PER_SOL,
            targetAdmin,
          }, [payer]);

          const [fundingPDA] = await client.getFundingRequestPDA(userPDA, payer.publicKey);
          const fundingRequest = await client.getFundingRequest(fundingPDA);
          
          result = {
            signature,
            fundingPDA: fundingPDA.toString(),
            fundingRequest
          };
          break;
        }

        case 'approve-funding': {
          // Этот пример требует предварительной настройки
          result = {
            message: 'Этот пример требует предварительной настройки админа и запроса финансирования'
          };
          break;
        }

        case 'dispatch-command': {
          const authority = Keypair.generate();
          const targetPubkey = Keypair.generate().publicKey;

          const pubkeyCommand = client.createPublishPubkeyCommand(targetPubkey);

          signature = await client.dispatchCommandUser({
            authority: authority.publicKey,
            targetPubkey,
            commandId: CMD_PUBLISH_PUBKEY,
            mode: CommandMode.OneWay,
            payload: pubkeyCommand,
          }, [authority]);

          result = {
            signature,
            commandId: CMD_PUBLISH_PUBKEY,
            mode: 'OneWay',
            targetPubkey: targetPubkey.toString()
          };
          break;
        }

        case 'connection-request': {
          const destination = {
            type: 'url' as const,
            url: 'https://example.com/bridge-endpoint'
          };

          const encryptedSessionKey = new Uint8Array(80);
          const config = client.createCommandConfig(
            12345,
            encryptedSessionKey,
            destination,
            new Uint8Array(0)
          );

          const payload = client.createRequestConnectionCommand(config);

          result = {
            config,
            payload: Array.from(payload),
            destination
          };
          break;
        }

        default:
          throw new Error('Неизвестный пример');
      }

      // Обновляем результат
      setResults(prev => prev.map(r => 
        r.id === exampleId 
          ? { ...r, status: 'success' as const, result, signature }
          : r
      ));

      toast.success(`Пример "${examples.find(e => e.id === exampleId)?.name}" выполнен успешно!`);

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Неизвестная ошибка';
      
      setResults(prev => prev.map(r => 
        r.id === exampleId 
          ? { ...r, status: 'error' as const, error: errorMessage }
          : r
      ));

      toast.error(`Ошибка в примере "${examples.find(e => e.id === exampleId)?.name}": ${errorMessage}`);
    } finally {
      setIsRunning(false);
      setSelectedExample(null);
    }
  };

  const runAllExamples = async () => {
    if (!client || !utils) {
      toast.error('Клиент не инициализирован');
      return;
    }

    setIsRunning(true);
    
    // Инициализируем результаты
    setResults(examples.map(example => ({
      id: example.id,
      name: example.name,
      status: 'pending' as const
    })));

    for (const example of examples) {
      await runExample(example.id);
      // Небольшая пауза между примерами
      await new Promise(resolve => setTimeout(resolve, 1000));
    }

    setIsRunning(false);
    toast.success('Все примеры выполнены!');
  };


  const getStatusIcon = (status: ExampleResultData['status']) => {
    switch (status) {
      case 'pending':
        return <div className="w-4 h-4 rounded-full bg-gray-400" />;
      case 'running':
        return <Loader2 className="w-4 h-4 animate-spin text-blue-500" />;
      case 'success':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'error':
        return <XCircle className="w-4 h-4 text-red-500" />;
    }
  };

  const getStatusText = (status: ExampleResultData['status']) => {
    switch (status) {
      case 'pending':
        return 'Ожидает';
      case 'running':
        return 'Выполняется';
      case 'success':
        return 'Успешно';
      case 'error':
        return 'Ошибка';
    }
  };

  if (!connection) {
    return (
      <div className="max-w-4xl mx-auto">
        <div className="text-center py-12">
          <Database className="mx-auto h-12 w-12 text-slate-400" />
          <h3 className="mt-2 text-sm font-medium text-slate-300">Подключение к Solana</h3>
          <p className="mt-1 text-sm text-slate-500">
            Подключитесь к кошельку для запуска примеров
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto">
      {/* Заголовок */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-white mb-2">Примеры использования Bridge клиента</h1>
        <p className="text-slate-400">
          Демонстрация всех возможностей W3B2 Bridge программы на Solana
        </p>
      </div>

      {/* Кнопки управления */}
      <div className="mb-6 flex flex-wrap gap-4">
        <button
          onClick={runAllExamples}
          disabled={isRunning}
          className="flex items-center space-x-2 px-4 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-slate-600 text-white rounded-lg transition-colors"
        >
          <Play className="w-4 h-4" />
          <span>Запустить все примеры</span>
        </button>
        
        <div className="flex items-center space-x-2 text-sm text-slate-400">
          <div className="w-2 h-2 bg-green-400 rounded-full"></div>
          <span>Подключено к {connection.rpcEndpoint}</span>
        </div>
      </div>

      {/* Статистика */}
      {results.length > 0 && (
        <div className="mb-8">
          <ExamplesStats results={results} isRunning={isRunning} />
        </div>
      )}

      {/* Сетка примеров */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {examples.map((example) => {
          const Icon = example.icon;
          const result = results.find(r => r.id === example.id);
          const isRunning = selectedExample === example.id;

          return (
            <div
              key={example.id}
              className="bg-slate-800 rounded-lg border border-slate-700 overflow-hidden"
            >
              {/* Заголовок карточки */}
              <div className="p-6 border-b border-slate-700">
                <div className="flex items-start justify-between">
                  <div className="flex items-center space-x-3">
                    <div className={`p-2 rounded-lg ${example.color}`}>
                      <Icon className="w-5 h-5 text-white" />
                    </div>
                    <div>
                      <h3 className="text-lg font-semibold text-white">{example.name}</h3>
                      <p className="text-sm text-slate-400 mt-1">{example.description}</p>
                    </div>
                  </div>
                  
                  <div className="flex items-center space-x-2">
                    {result && (
                      <div className="flex items-center space-x-1 text-sm">
                        {getStatusIcon(result.status)}
                        <span className="text-slate-300">{getStatusText(result.status)}</span>
                      </div>
                    )}
                    
                    <button
                      onClick={() => runExample(example.id)}
                      disabled={isRunning || (result?.status === 'running')}
                      className="flex items-center space-x-1 px-3 py-1 bg-slate-700 hover:bg-slate-600 disabled:bg-slate-600 text-white text-sm rounded transition-colors"
                    >
                      {isRunning ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <Play className="w-4 h-4" />
                      )}
                      <span>Запустить</span>
                    </button>
                  </div>
                </div>
              </div>

              {/* Код */}
              <div className="p-6">
                <CodeBlock
                  code={example.code}
                  language="typescript"
                  title="Код примера"
                  showCopyButton={true}
                  collapsible={true}
                  defaultExpanded={true}
                />
              </div>

              {/* Результат */}
              {result && (result.status === 'success' || result.status === 'error') && (
                <div className="p-6 border-t border-slate-700">
                  <ExampleResult
                    result={result.result}
                    signature={result.signature}
                    error={result.error}
                    status={result.status}
                  />
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Информация о программе */}
      <div className="mt-8">
        <ProgramInfo connection={connection} />
      </div>
    </div>
  );
};

export default Examples;
