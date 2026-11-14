import { useCallback, useState, type ReactNode } from 'react';
import { Check, Info, X, XCircle } from 'lucide-react';
import { ToastContext, type ToastMessage } from './toast-context';

export function ToastProvider({ children }: { children: ReactNode }) {
  const [messages, setMessages] = useState<ToastMessage[]>([]);
  const notify = useCallback((message: Omit<ToastMessage, 'id'>) => {
    const id = Date.now() + Math.random();
    setMessages((current) => [...current, { ...message, id }]);
    window.setTimeout(() => setMessages((current) => current.filter((entry) => entry.id !== id)), 4200);
  }, []);
  return (
    <ToastContext.Provider value={{ notify }}>
      {children}
      <div className="toast-region" aria-live="polite" aria-relevant="additions">
        {messages.map((message) => {
          const Icon = message.tone === 'success' ? Check : message.tone === 'error' ? XCircle : Info;
          return <div className={`toast toast-${message.tone}`} key={message.id}><Icon size={16} /><div><strong>{message.title}</strong>{message.detail && <p>{message.detail}</p>}</div><button className="toast-close" aria-label="Dismiss notification" onClick={() => setMessages((current) => current.filter((entry) => entry.id !== message.id))}><X size={14} /></button></div>;
        })}
      </div>
    </ToastContext.Provider>
  );
}
