// Presentation Hook — useToast
// Manages a self-dismissing toast notification queue.
// Used by SettingsView and ApiKeysView for per-action feedback.

import { useState, useCallback, useRef } from 'react';

export type ToastVariant = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  message: string;
  variant: ToastVariant;
}

let _id = 0;

export function useToast(autoDismissMs = 3500) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timers = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
    const timer = timers.current.get(id);
    if (timer !== undefined) {
      clearTimeout(timer);
      timers.current.delete(id);
    }
  }, []);

  const push = useCallback(
    (message: string, variant: ToastVariant = 'info') => {
      const id = ++_id;
      setToasts((prev) => [...prev, { id, message, variant }]);
      const timer = setTimeout(() => dismiss(id), autoDismissMs);
      timers.current.set(id, timer);
    },
    [autoDismissMs, dismiss],
  );

  return { toasts, push, dismiss };
}
