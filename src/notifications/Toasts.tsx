import { useEffect } from 'react';
import { useStore } from '../shared/store';
import type { Toast } from './notifications.slice';

const TTL_MS = 6000;

/** Transient notifications, stacked bottom-right. Errors stay until dismissed;
 *  the rest auto-expire. Clicking one that names a session focuses it. */
export function Toasts() {
  const toasts = useStore((s) => s.toasts);
  if (toasts.length === 0) return null;
  return (
    <div className="toasts">
      {toasts.map((t) => <ToastItem key={t.id} toast={t} />)}
    </div>
  );
}

function ToastItem({ toast }: { toast: Toast }) {
  const dismiss = useStore((s) => s.dismissToast);
  const setActiveSession = useStore((s) => s.setActiveSession);
  const setView = useStore((s) => s.setView);

  useEffect(() => {
    if (toast.kind === 'error') return;
    const h = setTimeout(() => dismiss(toast.id), TTL_MS);
    return () => clearTimeout(h);
  }, [toast.id, toast.kind, dismiss]);

  const focus = () => {
    if (toast.sessionId) { setActiveSession(toast.sessionId); setView('session'); }
    dismiss(toast.id);
  };

  return (
    <div className={`toast t-${toast.kind}${toast.sessionId ? ' clickable' : ''}`} onClick={focus}>
      <span className="toast-msg">{toast.message}</span>
      <button className="toast-x" onClick={(e) => { e.stopPropagation(); dismiss(toast.id); }}>×</button>
    </div>
  );
}
