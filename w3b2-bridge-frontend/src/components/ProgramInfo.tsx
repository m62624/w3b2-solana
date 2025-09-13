import React, { useState } from 'react';
import { 
  Info, 
  ExternalLink, 
  Copy, 
  CheckCircle, 
  Shield,
  Database,
  Activity,
  Code,
  Zap
} from 'lucide-react';
import toast from 'react-hot-toast';

interface ProgramInfoProps {
  connection: any;
}

const ProgramInfo: React.FC<ProgramInfoProps> = ({ connection }) => {
  const [copied, setCopied] = useState<string | null>(null);

  const programId = '3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr';
  const network = 'Solana Devnet';
  const rpcEndpoint = connection?.rpcEndpoint || 'Не подключено';

  const copyToClipboard = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
      toast.success(`${label} скопирован в буфер обмена!`);
      setTimeout(() => setCopied(null), 2000);
    } catch (error) {
      toast.error('Не удалось скопировать');
    }
  };

  const features = [
    {
      icon: Shield,
      title: 'Регистрация аккаунтов',
      description: 'Создание админских и пользовательских аккаунтов с PDA',
      color: 'text-blue-400'
    },
    {
      icon: Database,
      title: 'Система финансирования',
      description: 'Запросы и одобрение финансирования между пользователями',
      color: 'text-green-400'
    },
    {
      icon: Zap,
      title: 'Отправка команд',
      description: 'Двусторонняя и односторонняя отправка команд',
      color: 'text-purple-400'
    },
    {
      icon: Activity,
      title: 'Мониторинг событий',
      description: 'Отслеживание всех операций через события программы',
      color: 'text-orange-400'
    }
  ];

  const instructions = [
    {
      name: 'register_admin',
      description: 'Регистрация админского аккаунта',
      parameters: ['payer', 'authority', 'co_signer', 'initial_balance']
    },
    {
      name: 'register_user',
      description: 'Регистрация пользовательского аккаунта',
      parameters: ['payer', 'user_wallet', 'co_signer', 'initial_balance']
    },
    {
      name: 'deactivate_admin',
      description: 'Деактивация админского аккаунта',
      parameters: ['admin_account']
    },
    {
      name: 'deactivate_user',
      description: 'Деактивация пользовательского аккаунта',
      parameters: ['user_account']
    },
    {
      name: 'request_funding',
      description: 'Запрос финансирования',
      parameters: ['payer', 'user_account', 'amount', 'target_admin']
    },
    {
      name: 'approve_funding',
      description: 'Одобрение финансирования',
      parameters: ['admin_authority', 'funding_request', 'user_wallet']
    },
    {
      name: 'dispatch_command_admin',
      description: 'Отправка команды от админа',
      parameters: ['authority', 'admin_account', 'command_id', 'mode', 'payload', 'target_pubkey']
    },
    {
      name: 'dispatch_command_user',
      description: 'Отправка команды от пользователя',
      parameters: ['authority', 'user_account', 'command_id', 'mode', 'payload', 'target_pubkey']
    }
  ];

  return (
    <div className="space-y-6">
      {/* Основная информация */}
      <div className="bg-slate-800 rounded-lg border border-slate-700 p-6">
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center space-x-3">
            <div className="p-2 bg-blue-500 rounded-lg">
              <Shield className="w-6 h-6 text-white" />
            </div>
            <div>
              <h3 className="text-xl font-semibold text-white">W3B2 Bridge Program</h3>
              <p className="text-slate-400">Solana программа для Web3-to-Web2 моста</p>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-3">
            <div>
              <label className="text-sm font-medium text-slate-400">Программный ID</label>
              <div className="flex items-center space-x-2 mt-1">
                <code className="text-sm text-slate-300 font-mono bg-slate-900 px-2 py-1 rounded flex-1">
                  {programId}
                </code>
                <button
                  onClick={() => copyToClipboard(programId, 'Программный ID')}
                  className="text-slate-400 hover:text-white transition-colors"
                >
                  {copied === 'Программный ID' ? (
                    <CheckCircle className="w-4 h-4" />
                  ) : (
                    <Copy className="w-4 h-4" />
                  )}
                </button>
              </div>
            </div>

            <div>
              <label className="text-sm font-medium text-slate-400">Сеть</label>
              <div className="text-slate-300 mt-1">{network}</div>
            </div>
          </div>

          <div className="space-y-3">
            <div>
              <label className="text-sm font-medium text-slate-400">RPC Endpoint</label>
              <div className="text-slate-300 mt-1 break-all">{rpcEndpoint}</div>
            </div>

            <div>
              <label className="text-sm font-medium text-slate-400">Статус подключения</label>
              <div className="flex items-center space-x-2 mt-1">
                <div className="w-2 h-2 bg-green-400 rounded-full"></div>
                <span className="text-green-400">Активно</span>
              </div>
            </div>
          </div>
        </div>

        <div className="mt-4 pt-4 border-t border-slate-700">
          <a
            href={`https://explorer.solana.com/address/${programId}?cluster=devnet`}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center space-x-2 text-blue-400 hover:text-blue-300 transition-colors"
          >
            <ExternalLink className="w-4 h-4" />
            <span>Открыть в Solana Explorer</span>
          </a>
        </div>
      </div>

      {/* Возможности программы */}
      <div className="bg-slate-800 rounded-lg border border-slate-700 p-6">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center space-x-2">
          <Code className="w-5 h-5" />
          <span>Возможности программы</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {features.map((feature, index) => {
            const Icon = feature.icon;
            return (
              <div key={index} className="flex items-start space-x-3 p-3 bg-slate-900 rounded-lg">
                <Icon className={`w-5 h-5 ${feature.color} mt-0.5`} />
                <div>
                  <h4 className="font-medium text-white">{feature.title}</h4>
                  <p className="text-sm text-slate-400 mt-1">{feature.description}</p>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Инструкции программы */}
      <div className="bg-slate-800 rounded-lg border border-slate-700 p-6">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center space-x-2">
          <Database className="w-5 h-5" />
          <span>Инструкции программы</span>
        </h3>

        <div className="space-y-3">
          {instructions.map((instruction, index) => (
            <div key={index} className="p-4 bg-slate-900 rounded-lg">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <h4 className="font-mono text-blue-400 font-medium">
                    {instruction.name}
                  </h4>
                  <p className="text-slate-300 text-sm mt-1">
                    {instruction.description}
                  </p>
                  <div className="mt-2">
                    <span className="text-xs text-slate-400">Параметры:</span>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {instruction.parameters.map((param, paramIndex) => (
                        <span
                          key={paramIndex}
                          className="text-xs bg-slate-700 text-slate-300 px-2 py-1 rounded"
                        >
                          {param}
                        </span>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Техническая информация */}
      <div className="bg-slate-800 rounded-lg border border-slate-700 p-6">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center space-x-2">
          <Info className="w-5 h-5" />
          <span>Техническая информация</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
          <div className="space-y-2">
            <div className="flex justify-between">
              <span className="text-slate-400">Версия Anchor:</span>
              <span className="text-slate-300">0.29.0</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Версия Solana:</span>
              <span className="text-slate-300">1.17.0</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Размер программы:</span>
              <span className="text-slate-300">~50KB</span>
            </div>
          </div>
          
          <div className="space-y-2">
            <div className="flex justify-between">
              <span className="text-slate-400">Максимальный payload:</span>
              <span className="text-slate-300">1024 байта</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Поддерживаемые команды:</span>
              <span className="text-slate-300">2 типа</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">События программы:</span>
              <span className="text-slate-300">7 типов</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ProgramInfo;
