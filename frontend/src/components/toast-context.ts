import { createContext } from 'react';

export interface ToastMessage {
  id: number;
  title: string;
  detail?: string;
  tone: 'success' | 'info' | 'error';
}

export interface ToastContextValue {
  notify: (message: Omit<ToastMessage, 'id'>) => void;
}

export const ToastContext = createContext<ToastContextValue | null>(null);
