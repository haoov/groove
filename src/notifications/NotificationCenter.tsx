import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  AlertTriangle, Bell, Bot, Check, CheckCircle2, Copy, Expand, FileText, GitBranch,
  GitPullRequest, Info, Plug, StickyNote, X, type LucideIcon,
} from 'lucide-react';
import { goToSession } from '../shared/lib/goToSession';
import { useStore } from '../shared/store';
import type { AppNotification, NotificationSource } from '../shared/store';

/**
 * The reviewable view of the notification feed.
 *
 * Everything the app announces lands here — agent attention, MCP approvals, git
 * and forge outcomes, failures that used to be a single overwritten `lastError`.
 * Toasts are the same records shown transiently; this is where they persist until
 * cleared, which is what makes an interruption you missed still recoverable.
 */

const KIND_ICON: Record<AppNotification['kind'], LucideIcon> = {
  success: CheckCircle2,
  error: AlertTriangle,
  attention: Bell,
  info: Info,
};

const SOURCE_ICON: Record<NotificationSource, LucideIcon> = {
  agent: Bot,
  mcp: Plug,
  git: GitBranch,
  mr: GitPullRequest,
  notion: StickyNote,
  files: FileText,
  app: Info,
};

function relativeTime(at: number): string {
  const s = Math.max(0, Math.round((Date.now() - at) / 1000));
  if (s < 45) return 'just now';
  const m = Math.round(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.round(h / 24)}d ago`;
}

/** One entry, shared by the toast stack and the panel so they can't drift. */
export function NotificationRow({
  n, variant, onDismiss, onExpand,
}: {
  n: AppNotification;
  variant: 'toast' | 'feed';
  onDismiss?: () => void;
  /** Offered in the feed: the detail is clamped to three lines there, and an error
   *  is exactly the kind of message whose last line matters. */
  onExpand?: () => void;
}) {
  const KindIcon = KIND_ICON[n.kind];
  const SourceIcon = SOURCE_ICON[n.source ?? 'app'];

  /** Navigate when the event says where it belongs. */
  const go = () => {
    if (!n.goTo?.taskId) return;
    // Shared with the activity rail's agent list — including the desk case.
    if (goToSession(n.goTo.taskId, { agent: n.goTo.agent })) {
      useStore.getState().setNotificationsOpen(false);
    }
  };

  // In the feed, acting on a row is what marks it seen — clicking it counts,
  // whether or not it has somewhere to take you.
  const onRowClick =
    variant === 'feed'
      ? () => {
          useStore.getState().markNotificationRead(n.id);
          go();
        }
      : undefined;

  const unread = variant === 'feed' && !n.read;
  return (
    <div
      className={`notif notif--${n.kind} notif--${variant} ${onRowClick ? 'clickable' : ''} ${unread ? 'unread' : ''}`}
      role={variant === 'toast' ? 'status' : undefined}
      onClick={onRowClick}
    >
      <KindIcon className="notif-icon" size={variant === 'toast' ? 18 : 15} strokeWidth={2} />
      <div className="notif-main">
        <div className="notif-title">
          {n.title}
          {n.count > 1 && <span className="notif-count">×{n.count}</span>}
        </div>
        {n.detail && <div className="notif-detail">{n.detail}</div>}
        <div className="notif-meta">
          <SourceIcon size={10} strokeWidth={2} />
          <span>{n.source ?? 'app'}</span>
          {n.taskId && <span className="notif-chip">{n.taskId}</span>}
          {n.repo && <span className="notif-chip repo">{n.repo}</span>}
          <span className="notif-time">{relativeTime(n.at)}</span>
        </div>
      </div>
      {onExpand && (
        <button
          className="notif-expand"
          title="Show in full"
          onClick={(e) => { e.stopPropagation(); onExpand(); }}
        >
          <Expand size={12} strokeWidth={2.25} />
        </button>
      )}
      {unread && (
        <button
          className="notif-seen"
          title="Mark as seen"
          onClick={(e) => { e.stopPropagation(); useStore.getState().markNotificationRead(n.id); }}
        >
          <Check size={13} strokeWidth={2.25} />
        </button>
      )}
      {onDismiss && (
        <button
          className="notif-close"
          title="Dismiss"
          onClick={(e) => { e.stopPropagation(); onDismiss(); }}
        >
          <X size={13} strokeWidth={2.25} />
        </button>
      )}
    </div>
  );
}

/** The feed, as the session dock's second tab. */
export function NotificationFeed() {
  const all = useStore((s) => s.notifications);
  const clear = useStore((s) => s.clearNotifications);
  const markAllRead = useStore((s) => s.markNotificationsRead);
  // A snapshot, not an id: the point is to read a message that is no longer
  // changing, and clearing the feed behind the modal must not empty it.
  const [expanded, setExpanded] = useState<AppNotification | null>(null);
  // Successes are ephemeral: they flash past as a toast and belong to neither the
  // history nor the count. Filtering in one place keeps the two in agreement.
  const notifications = all.filter((n) => !n.ephemeral);
  const unread = notifications.filter((n) => !n.read).length;

  return (
    <div className="dock-notifs">
      {notifications.length > 0 && (
        <div className="dock-notifs-actions">
          {unread > 0 && <button className="dock-notifs-action" onClick={markAllRead}>mark all seen</button>}
          <button className="dock-notifs-action" onClick={clear}>clear</button>
        </div>
      )}
      {notifications.length === 0 ? (
        <p className="dock-empty">Nothing to report.</p>
      ) : (
        <div className="notif-list">
          {notifications.map((n) => (
            <NotificationRow
              key={n.id}
              n={n}
              variant="feed"
              onExpand={() => {
                useStore.getState().markNotificationRead(n.id);
                setExpanded(n);
              }}
            />
          ))}
        </div>
      )}
      {expanded && <NotificationModal n={expanded} onClose={() => setExpanded(null)} />}
    </div>
  );
}

/**
 * One notification, in full.
 *
 * The feed clamps a detail to three lines, which is fine for "pushed 3 commits" and
 * useless for a git or forge error — the line that says what went wrong is usually
 * the last one. So this shows the whole thing, selectable, and offers it to the
 * clipboard: an error's next stop is a search box or a colleague.
 */
function NotificationModal({ n, onClose }: { n: AppNotification; onClose: () => void }) {
  const KindIcon = KIND_ICON[n.kind];
  const [copied, setCopied] = useState(false);

  const copy = () => {
    // Through the backend, like the terminals: WebKitGTK has no clipboard API
    // outside a secure context (see components/terminalHost.ts).
    invoke('copy_to_clipboard', { text: [n.title, n.detail ?? ''].join('\n\n').trim() })
      .then(() => setCopied(true))
      .catch((e) => useStore.getState().setLastError(String(e)));
  };

  return (
    <div className="wizard-overlay" onClick={onClose}>
      <div className={`wizard-modal notif-modal notif--${n.kind}`} onClick={(e) => e.stopPropagation()}>
        <div className="wizard-header">
          <div className="wizard-title">
            <KindIcon className="notif-icon" size={15} strokeWidth={2} />
            {n.title}
          </div>
          <div className="wizard-subtitle notif-modal-meta">
            <span>{n.source ?? 'app'}</span>
            {n.taskId && <span className="notif-chip">{n.taskId}</span>}
            {n.repo && <span className="notif-chip repo">{n.repo}</span>}
            <span className="notif-time">{relativeTime(n.at)}</span>
            {n.count > 1 && <span className="notif-count">×{n.count}</span>}
          </div>
          <button className="wizard-close" onClick={onClose}>×</button>
        </div>
        {n.detail
          ? <pre className="notif-modal-detail">{n.detail}</pre>
          : <p className="notif-modal-empty">No further detail was reported.</p>}
        <div className="wizard-footer">
          <button className="btn-secondary" onClick={copy}>
            {copied
              ? <><Check size={11} strokeWidth={2} style={{ marginRight: 5 }} />Copied</>
              : <><Copy size={11} strokeWidth={2} style={{ marginRight: 5 }} />Copy</>}
          </button>
          <span className="composer-spacer" />
          <button className="btn-primary" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

/** The header bell: the unread count, and the way to the dock's feed. */
export function NotificationCenter() {
  const all = useStore((s) => s.notifications);
  const setOpen = useStore((s) => s.setNotificationsOpen);
  const setDockOpen = useStore((s) => s.setDockOpen);
  const unread = all.filter((n) => !n.ephemeral && !n.read).length;

  return (
    <button
      className={`notif-bell ${unread > 0 ? 'has-unread' : ''}`}
      onClick={() => { setDockOpen(true); setOpen(true); }}
      title={unread > 0 ? `${unread} new notification${unread === 1 ? '' : 's'}` : 'Notifications (Ctrl+N)'}
    >
      <Bell size={14} strokeWidth={1.75} />
      {unread > 0 && <span className="notif-bell-badge">{unread > 9 ? '9+' : unread}</span>}
    </button>
  );
}
