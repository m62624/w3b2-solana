import React, { useState } from 'react';
import { 
  CheckCircle, 
  XCircle, 
  ExternalLink, 
  Copy, 
  ChevronDown, 
  ChevronRight,
  Info
} from 'lucide-react';
import toast from 'react-hot-toast';

interface ExampleResultProps {
  result: any;
  signature?: string;
  error?: string;
  status: 'success' | 'error';
}

const ExampleResult: React.FC<ExampleResultProps> = ({ 
  result, 
  signature, 
  error, 
  status 
}) => {
  const [expanded, setExpanded] = useState(false);

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    toast.success('Скопировано в буфер обмена!');
  };

  const formatValue = (value: any): string => {
    if (typeof value === 'string') {
      return value;
    }
    if (typeof value === 'number') {
      return value.toString();
    }
    if (value instanceof Uint8Array) {
      return `Uint8Array(${value.length})`;
    }
    if (Array.isArray(value)) {
      return `Array(${value.length})`;
    }
    if (typeof value === 'object' && value !== null) {
      return JSON.stringify(value, null, 2);
    }
    return String(value);
  };

  const renderValue = (key: string, value: any) => {
    if (key.toLowerCase().includes('signature') || key.toLowerCase().includes('pda')) {
      return (
        <div className="flex items-center space-x-2">
          <span className="font-mono text-xs text-slate-300 break-all">
            {formatValue(value)}
          </span>
          <button
            onClick={() => copyToClipboard(formatValue(value))}
            className="text-slate-400 hover:text-white transition-colors"
          >
            <Copy className="w-3 h-3" />
          </button>
        </div>
      );
    }

    if (typeof value === 'object' && value !== null) {
      return (
        <div className="bg-slate-900 rounded p-2">
          <pre className="text-xs text-slate-300 overflow-x-auto">
            {JSON.stringify(value, null, 2)}
          </pre>
        </div>
      );
    }

    return (
      <span className="text-slate-300 font-mono text-sm">
        {formatValue(value)}
      </span>
    );
  };

  if (status === 'error') {
    return (
      <div className="bg-red-900/20 border border-red-500/20 rounded-lg p-4">
        <div className="flex items-center space-x-2 mb-2">
          <XCircle className="w-5 h-5 text-red-400" />
          <h4 className="text-red-400 font-medium">Ошибка выполнения</h4>
        </div>
        <p className="text-red-300 text-sm">{error}</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Подпись транзакции */}
      {signature && (
        <div className="bg-slate-800 rounded-lg p-4">
          <div className="flex items-center justify-between mb-2">
            <h4 className="text-sm font-medium text-slate-300 flex items-center space-x-2">
              <CheckCircle className="w-4 h-4 text-green-400" />
              <span>Подпись транзакции</span>
            </h4>
            <button
              onClick={() => copyToClipboard(signature)}
              className="text-slate-400 hover:text-white transition-colors"
            >
              <Copy className="w-4 h-4" />
            </button>
          </div>
          <div className="font-mono text-xs text-slate-300 break-all bg-slate-900 rounded p-2">
            {signature}
          </div>
          <a
            href={`https://explorer.solana.com/tx/${signature}?cluster=devnet`}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center space-x-1 text-xs text-blue-400 hover:text-blue-300 mt-2 transition-colors"
          >
            <ExternalLink className="w-3 h-3" />
            <span>Открыть в Solana Explorer</span>
          </a>
        </div>
      )}

      {/* Результат */}
      {result && (
        <div className="bg-slate-800 rounded-lg p-4">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-sm font-medium text-slate-300 flex items-center space-x-2">
              <Info className="w-4 h-4 text-blue-400" />
              <span>Результат выполнения</span>
            </h4>
            <button
              onClick={() => setExpanded(!expanded)}
              className="text-slate-400 hover:text-white transition-colors"
            >
              {expanded ? (
                <ChevronDown className="w-4 h-4" />
              ) : (
                <ChevronRight className="w-4 h-4" />
              )}
            </button>
          </div>

          {expanded ? (
            <div className="space-y-3">
              {Object.entries(result).map(([key, value]) => (
                <div key={key} className="space-y-1">
                  <div className="text-xs font-medium text-slate-400 capitalize">
                    {key.replace(/([A-Z])/g, ' $1').trim()}:
                  </div>
                  {renderValue(key, value)}
                </div>
              ))}
            </div>
          ) : (
            <div className="text-sm text-slate-400">
              Нажмите для просмотра деталей результата
            </div>
          )}
        </div>
      )}

      {/* Краткая сводка */}
      <div className="bg-green-900/20 border border-green-500/20 rounded-lg p-4">
        <div className="flex items-center space-x-2">
          <CheckCircle className="w-5 h-5 text-green-400" />
          <span className="text-green-300 font-medium">Пример выполнен успешно!</span>
        </div>
        <p className="text-green-200 text-sm mt-1">
          Все операции были выполнены корректно и записаны в блокчейн.
        </p>
      </div>
    </div>
  );
};

export default ExampleResult;
