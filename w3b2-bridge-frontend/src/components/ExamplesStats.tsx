import React from 'react';
import { 
  CheckCircle, 
  XCircle, 
  Clock, 
  Play,
  TrendingUp,
  Activity
} from 'lucide-react';

interface ExampleResultData {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'success' | 'error';
  result?: any;
  error?: string;
  signature?: string;
}

interface ExamplesStatsProps {
  results: ExampleResultData[];
  isRunning: boolean;
}

const ExamplesStats: React.FC<ExamplesStatsProps> = ({ results, isRunning }) => {
  const total = results.length;
  const completed = results.filter(r => r.status === 'success' || r.status === 'error').length;
  const successful = results.filter(r => r.status === 'success').length;
  const failed = results.filter(r => r.status === 'error').length;
  const pending = results.filter(r => r.status === 'pending').length;
  const running = results.filter(r => r.status === 'running').length;

  const successRate = total > 0 ? Math.round((successful / total) * 100) : 0;
  const completionRate = total > 0 ? Math.round((completed / total) * 100) : 0;

  const stats = [
    {
      label: 'Всего примеров',
      value: total,
      icon: Activity,
      color: 'text-slate-400',
      bgColor: 'bg-slate-800'
    },
    {
      label: 'Выполнено',
      value: completed,
      icon: CheckCircle,
      color: 'text-blue-400',
      bgColor: 'bg-blue-900/20'
    },
    {
      label: 'Успешно',
      value: successful,
      icon: CheckCircle,
      color: 'text-green-400',
      bgColor: 'bg-green-900/20'
    },
    {
      label: 'С ошибками',
      value: failed,
      icon: XCircle,
      color: 'text-red-400',
      bgColor: 'bg-red-900/20'
    },
    {
      label: 'Ожидают',
      value: pending,
      icon: Clock,
      color: 'text-yellow-400',
      bgColor: 'bg-yellow-900/20'
    },
    {
      label: 'Выполняются',
      value: running,
      icon: Play,
      color: 'text-purple-400',
      bgColor: 'bg-purple-900/20'
    }
  ];

  return (
    <div className="bg-slate-800 rounded-lg border border-slate-700 p-6">
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-lg font-semibold text-white flex items-center space-x-2">
          <TrendingUp className="w-5 h-5" />
          <span>Статистика выполнения</span>
        </h3>
        
        {isRunning && (
          <div className="flex items-center space-x-2 text-sm text-blue-400">
            <div className="w-2 h-2 bg-blue-400 rounded-full animate-pulse"></div>
            <span>Выполняется...</span>
          </div>
        )}
      </div>

      {/* Основные метрики */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-6">
        {stats.map((stat, index) => {
          const Icon = stat.icon;
          return (
            <div
              key={index}
              className={`${stat.bgColor} rounded-lg p-4 border border-slate-700`}
            >
              <div className="flex items-center space-x-2 mb-2">
                <Icon className={`w-4 h-4 ${stat.color}`} />
                <span className={`text-sm font-medium ${stat.color}`}>
                  {stat.label}
                </span>
              </div>
              <div className="text-2xl font-bold text-white">
                {stat.value}
              </div>
            </div>
          );
        })}
      </div>

      {/* Прогресс-бары */}
      <div className="space-y-4">
        {/* Общий прогресс */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-slate-300">Общий прогресс</span>
            <span className="text-sm text-slate-400">{completionRate}%</span>
          </div>
          <div className="w-full bg-slate-700 rounded-full h-2">
            <div
              className="bg-blue-500 h-2 rounded-full transition-all duration-300"
              style={{ width: `${completionRate}%` }}
            />
          </div>
        </div>

        {/* Успешность */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-slate-300">Успешность</span>
            <span className="text-sm text-slate-400">{successRate}%</span>
          </div>
          <div className="w-full bg-slate-700 rounded-full h-2">
            <div
              className="bg-green-500 h-2 rounded-full transition-all duration-300"
              style={{ width: `${successRate}%` }}
            />
          </div>
        </div>
      </div>

      {/* Детальная информация */}
      {completed > 0 && (
        <div className="mt-6 pt-6 border-t border-slate-700">
          <h4 className="text-sm font-medium text-slate-300 mb-3">Детали выполнения</h4>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-slate-400">Успешных операций:</span>
                <span className="text-green-400 font-medium">{successful}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400">Неудачных операций:</span>
                <span className="text-red-400 font-medium">{failed}</span>
              </div>
            </div>
            
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-slate-400">Средняя успешность:</span>
                <span className="text-blue-400 font-medium">{successRate}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400">Общий прогресс:</span>
                <span className="text-purple-400 font-medium">{completionRate}%</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Рекомендации */}
      {failed > 0 && (
        <div className="mt-6 p-4 bg-yellow-900/20 border border-yellow-500/20 rounded-lg">
          <div className="flex items-start space-x-2">
            <XCircle className="w-5 h-5 text-yellow-400 mt-0.5" />
            <div>
              <h4 className="text-yellow-400 font-medium text-sm">Рекомендации</h4>
              <p className="text-yellow-200 text-sm mt-1">
                Некоторые примеры завершились с ошибками. Проверьте подключение к сети 
                и убедитесь, что у вас достаточно SOL для выполнения транзакций.
              </p>
            </div>
          </div>
        </div>
      )}

      {successful === total && total > 0 && (
        <div className="mt-6 p-4 bg-green-900/20 border border-green-500/20 rounded-lg">
          <div className="flex items-start space-x-2">
            <CheckCircle className="w-5 h-5 text-green-400 mt-0.5" />
            <div>
              <h4 className="text-green-400 font-medium text-sm">Отлично!</h4>
              <p className="text-green-200 text-sm mt-1">
                Все примеры выполнены успешно! Вы можете изучить результаты 
                и использовать код в своих проектах.
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default ExamplesStats;
