import { useEffect } from 'react';
import { useStore } from '../shared/store';
import { NotificationRow } from './NotificationCenter';
import type { AppNotification } from '../shared/store';

/** Errors stay up longer — you may be reading them, not glancing. */
const DISMISS_MS: Record<AppNotification['kind'], number> = {
  success: 3800,
  info: 3800,
  attention: 7000,
  error: 8000,
};

function Toast({ n }: { n: AppNotification }) {
  const dismiss = useStore((s) => s.dismissToast);
  useEffect(() => {
    const t = window.setTimeout(() => dismiss(n.id), DISMISS_MS[n.kind]);
    return () => clearTimeout(t);
  }, [n.id, n.kind, dismiss]);

  return <NotificationRow n={n} variant="toast" onDismiss={() => dismiss(n.id)} />;
}

/**
 * The interrupting view of the notification feed, top right.
 *
 * Dismissing one only hides it here — the entry stays in the notification centre,
 * so nothing announced becomes unrecoverable.
 */
export function Toasts() {
  const notifications = useStore((s) => s.notifications);
  const toastIds = useStore((s) => s.toastIds);
  const panelOpen = useStore((s) => s.notificationsOpen);

  // No point shouting at someone who is already reading the feed.
  if (panelOpen) return null;

  const shown = toastIds
    .map((id) => notifications.find((n) => n.id === id))
    .filter((n): n is AppNotification => !!n);
  if (shown.length === 0) return null;

  return (
    <div className="toast-stack">
      {shown.map((n) => <Toast key={n.id} n={n} />)}
    </div>
  );
}
