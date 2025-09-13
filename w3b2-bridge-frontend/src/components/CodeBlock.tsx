import React, { useState } from 'react';
import { Copy, Check, ChevronDown, ChevronRight } from 'lucide-react';
import toast from 'react-hot-toast';

interface CodeBlockProps {
  code: string;
  language?: string;
  title?: string;
  showCopyButton?: boolean;
  collapsible?: boolean;
  defaultExpanded?: boolean;
}

const CodeBlock: React.FC<CodeBlockProps> = ({
  code,
  language = 'typescript',
  title,
  showCopyButton = true,
  collapsible = false,
  defaultExpanded = true
}) => {
  const [copied, setCopied] = useState(false);
  const [expanded, setExpanded] = useState(defaultExpanded);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      toast.success('Код скопирован в буфер обмена!');
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      toast.error('Не удалось скопировать код');
    }
  };

  const toggleExpanded = () => {
    setExpanded(!expanded);
  };

  // Простая подсветка синтаксиса для TypeScript/JavaScript
  const highlightCode = (code: string) => {
    return code
      .replace(/\b(const|let|var|function|async|await|return|if|else|for|while|switch|case|break|continue|try|catch|finally|throw|new|class|interface|type|enum|import|export|from|default|as|typeof|instanceof|in|of|extends|implements|public|private|protected|static|readonly|abstract|override|super|this)\b/g, 
        '<span class="text-blue-400 font-semibold">$1</span>')
      .replace(/\b(true|false|null|undefined)\b/g, 
        '<span class="text-purple-400">$1</span>')
      .replace(/\b(\d+\.?\d*)\b/g, 
        '<span class="text-green-400">$1</span>')
      .replace(/(['"`])((?:\\.|(?!\1)[^\\])*?)\1/g, 
        '<span class="text-yellow-300">$1$2$1</span>')
      .replace(/(\/\/.*$)/gm, 
        '<span class="text-slate-500 italic">$1</span>')
      .replace(/(\/\*[\s\S]*?\*\/)/g, 
        '<span class="text-slate-500 italic">$1</span>')
      .replace(/\b(console\.log|console\.error|console\.warn|console\.info)\b/g, 
        '<span class="text-orange-400">$1</span>')
      .replace(/\b(Keypair|PublicKey|Connection|Transaction|TransactionInstruction|SystemProgram|sendAndConfirmTransaction)\b/g, 
        '<span class="text-cyan-400 font-semibold">$1</span>')
      .replace(/\b(BridgeClient|BridgeUtils|CommandMode|FundingStatus|CMD_PUBLISH_PUBKEY|CMD_REQUEST_CONNECTION)\b/g, 
        '<span class="text-pink-400 font-semibold">$1</span>');
  };

  return (
    <div className="bg-slate-800 rounded-lg border border-slate-700 overflow-hidden">
      {/* Заголовок */}
      {(title || showCopyButton || collapsible) && (
        <div className="flex items-center justify-between p-4 border-b border-slate-700">
          <div className="flex items-center space-x-2">
            {collapsible && (
              <button
                onClick={toggleExpanded}
                className="text-slate-400 hover:text-white transition-colors"
              >
                {expanded ? (
                  <ChevronDown className="w-4 h-4" />
                ) : (
                  <ChevronRight className="w-4 h-4" />
                )}
              </button>
            )}
            {title && (
              <h4 className="text-sm font-medium text-slate-300">{title}</h4>
            )}
            {!title && collapsible && (
              <span className="text-sm text-slate-400">
                {language.toUpperCase()} код
              </span>
            )}
          </div>
          
          {showCopyButton && (
            <button
              onClick={handleCopy}
              className="flex items-center space-x-1 text-slate-400 hover:text-white transition-colors text-sm"
            >
              {copied ? (
                <>
                  <Check className="w-4 h-4" />
                  <span>Скопировано</span>
                </>
              ) : (
                <>
                  <Copy className="w-4 h-4" />
                  <span>Копировать</span>
                </>
              )}
            </button>
          )}
        </div>
      )}

      {/* Код */}
      {(expanded || !collapsible) && (
        <div className="relative">
          <div className="bg-slate-900 p-4 overflow-x-auto">
            <pre className="text-sm text-slate-300 font-mono leading-relaxed">
              <code 
                dangerouslySetInnerHTML={{ 
                  __html: highlightCode(code) 
                }}
              />
            </pre>
          </div>
          
          {/* Информация о языке */}
          <div className="absolute top-2 right-2">
            <span className="text-xs text-slate-500 bg-slate-800 px-2 py-1 rounded">
              {language}
            </span>
          </div>
        </div>
      )}

      {/* Счетчик строк */}
      {expanded && (
        <div className="px-4 py-2 bg-slate-900 border-t border-slate-700">
          <div className="flex items-center justify-between text-xs text-slate-500">
            <span>
              {code.split('\n').length} строк{code.split('\n').length !== 1 ? 'и' : 'а'}
            </span>
            <span>
              {code.length} символов
            </span>
          </div>
        </div>
      )}
    </div>
  );
};

export default CodeBlock;
