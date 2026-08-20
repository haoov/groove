import { useEffect, useMemo, useRef, useState } from 'react';
import { Code2, Compass, GitPullRequest, MessageSquare, X, type LucideIcon } from 'lucide-react';
import { useStore, type SessionKind, type SessionState } from '../shared/store';
import { endSession } from '../app/providers/useIpc';
import { NotificationFeed } from '../notifications/NotificationCenter';
import { goToSession } from '../shared/lib/goToSession';
import { statusKey } from '../shared/lib/taskStatus';
import type { AgentActivity } from '../shared/ipc/ipc';

/**
 * The open sessions, on the right, with what each agent is doing.
 *
 * This is the ONLY list of sessions: it replaced the header tabs (switch, close),
 * the activity rail's agent list (state, jump) and the Alt+S modal (keyboard
 * switching). Three renderings of the same data drift; one does not.
 *
 * App-level rather than per-session, so it survives view changes and lists the
 * same sessions on Home. The desk is not among them: it is not something you
 * opened, it cannot be closed, and it is reached through the console on Home.
 *
 * Every value is read from the store (sessions, agentActivity). Nothing here
 * fetches, so it can re-render as freely as the agents report.
 */

const MIN_WIDTH = 200;
const MAX_WIDTH = 460;
const DEFAULT_WIDTH = 260;
const WIDTH_KEY = 'wb.dockWidth';

const KIND_LABEL: Record<SessionKind, string> = {
  task: 'task',
  explorer: 'expl',
  review: 'review',
  desk: 'desk',
};

const KIND_ICON: Record<SessionKind, LucideIcon> = {
  task: Code2,
  explorer: Compass,
  review: GitPullRequest,
  desk: MessageSquare,
};

export function SessionDock() {
  const open = useStore((s) => s.dockOpen);
  const focusNonce = useStore((s) => s.dockFocusNonce);
  const setDockOpen = useStore((s) => s.setDockOpen);
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const activeSessionId = useStore((s) => s.activeSessionId);
  const activity = useStore((s) => s.agentActivity);
  // The notification feed is the dock's second tab rather than a separate popover:
  // one panel on the right, two things you check there.
  const showNotifications = useStore((s) => s.notificationsOpen);
  const setNotificationsOpen = useStore((s) => s.setNotificationsOpen);
  const unread = useStore((s) => s.notifications.filter((n) => !n.ephemeral && !n.read).length);

  const [width, setWidth] = useState(() => {
    const saved = Number(localStorage.getItem(WIDTH_KEY));
    return Number.isFinite(saved) && saved >= MIN_WIDTH ? saved : DEFAULT_WIDTH;
  });
  const [cursor, setCursor] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const asideRef = useRef<HTMLElement>(null);

  // The desk is excluded: it is not a session you opened, it cannot be closed,
  // and it is reached through the console on Home. Its agent still reports through
  // notifications.
  const rows = useMemo(
    () => sessionOrder
      .map((id) => sessions[id])
      .filter((s): s is SessionState => !!s && s.kind !== 'desk'),
    [sessionOrder, sessions],
  );

  // Take keyboard focus, starting on the session you are already in so Enter is a
  // no-op rather than a jump. Which TAB to show is the caller's call (the two
  // shortcuts want different ones), so this only moves focus — into whichever list
  // is on screen, since both shortcuts also need a second press to close the dock.
  useEffect(() => {
    if (!focusNonce) return;
    const at = rows.findIndex((s) => s.id === activeSessionId);
    setCursor(at >= 0 ? at : 0);
    listRef.current?.focus();
  }, [focusNonce]); // eslint-disable-line react-hooks/exhaustive-deps

  const onKeyDown = (e: React.KeyboardEvent) => {
    const down = e.key === 'j' || e.key === 'ArrowDown';
    const up = e.key === 'k' || e.key === 'ArrowUp';
    if (down || up) {
      e.preventDefault();
      setCursor((c) => (rows.length ? (c + (down ? 1 : -1) + rows.length) % rows.length : 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const pick = rows[cursor];
      if (pick?.task) goToSession(pick.task.short_id);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      // Hand the keyboard back to the work, not to the browser's focus order.
      listRef.current?.blur();
      useStore.getState().requestPanelFocus();
    }
  };

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    // The rendered width, not the stored one: in a window too narrow for every
    // column this has been shrunk, and dragging from the stored value would jump.
    const startWidth = asideRef.current?.getBoundingClientRect().width ?? width;
    let latest = startWidth;
    const move = (ev: MouseEvent) => {
      // Dragging LEFT widens: the handle is on the dock's inner edge.
      latest = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth + (startX - ev.clientX)));
      setWidth(latest);
    };
    const up = () => {
      document.removeEventListener('mousemove', move);
      document.removeEventListener('mouseup', up);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      localStorage.setItem(WIDTH_KEY, String(Math.round(latest)));
    };
    document.addEventListener('mousemove', move);
    document.addEventListener('mouseup', up);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  };

  if (!open) return null;

  return (
    <>
      <div className="resize-handle" onMouseDown={startDrag} />
      <aside className="session-dock" ref={asideRef} style={{ width, minWidth: MIN_WIDTH }}>
        <div className="dock-head">
          <button
            className={`dock-tab ${showNotifications ? '' : 'on'}`}
            onClick={() => setNotificationsOpen(false)}
          >
            Active
            <span className="dock-count">{rows.length}</span>
          </button>
          <button
            className={`dock-tab ${showNotifications ? 'on' : ''}`}
            onClick={() => setNotificationsOpen(true)}
            title="Notifications (Ctrl+N)"
          >
            Notifications
            {unread > 0 && <span className="dock-count unread">{unread}</span>}
          </button>
          <button className="dock-close" onClick={() => setDockOpen(false)} title="Hide the dock (Alt+S reopens)">
            <X size={12} strokeWidth={2} />
          </button>
        </div>

        {showNotifications ? (
          // Focusable, because the notification shortcut closes the dock on a second
          // press — which it can only know from focus being in here.
          <div className="dock-feed" ref={listRef} tabIndex={-1}>
            <NotificationFeed />
          </div>
        ) : (
          <div
            className="dock-list"
            ref={listRef}
            tabIndex={-1}
            onKeyDown={onKeyDown}
          >
            {rows.length === 0 ? (
              <p className="dock-empty">No sessions open.</p>
            ) : (
              rows.map((s, i) => (
                <DockRow
                  key={s.id}
                  session={s}
                  activity={s.task ? activity[s.task.short_id] : undefined}
                  active={s.id === activeSessionId}
                  cursor={i === cursor}
                />
              ))
            )}
          </div>
        )}
      </aside>
    </>
  );
}

function DockRow({
  session, activity, active, cursor,
}: {
  session: SessionState;
  activity: AgentActivity | undefined;
  active: boolean;
  cursor: boolean;
}) {
  const Icon = KIND_ICON[session.kind] ?? Code2;
  const taskId = session.task?.short_id;
  // Title first, like a commit message: the id is a lookup key, the title is what
  // you actually recognise the session by.
  const title = session.task?.title || session.title;
  const status = session.task?.status;

  return (
    <div className={`dock-row ${active ? 'active' : ''} ${cursor ? 'cursor' : ''}`}>
      <button
        className="dock-row-main"
        onClick={() => taskId && goToSession(taskId)}
        title={`${title}\n${taskId ?? ''}`}
      >
        <span className="dock-row-title">{title}</span>
        {/* Meta wraps rather than truncating: the dock is narrow, and a clipped
            status or agent state is worse than a second line. */}
        <span className="dock-row-meta">
          <Icon size={11} strokeWidth={1.75} className="dock-row-icon" />
          {taskId && <span className="dock-row-id">{taskId}</span>}
          <span className="dock-row-kind">{KIND_LABEL[session.kind]}</span>
          {activity ? (
            <span className={`dock-row-state ${activity.state}`}>
              <span className={`pill-dot ${activity.state}`} />
              {agentLine(activity)}
            </span>
          ) : status ? (
            <span className={`dock-row-status status-${statusKey(status)}`}>{status}</span>
          ) : null}
        </span>
      </button>

      {/* Closing was the tabs' job; the dock inherits it. */}
      <button
        className="dock-row-close"
        title="Close session"
        onClick={(e) => { e.stopPropagation(); endSession(session.id); }}
      >
        <X size={11} strokeWidth={2.25} />
      </button>
    </div>
  );
}

/** What the agent is doing, in one line — the reason a row beats a tab. */
function agentLine(a: AgentActivity): string {
  const tool = a.tool ? (a.tool.detail ? `${a.tool.name}(${a.tool.detail})` : a.tool.name) : null;
  switch (a.state) {
    case 'waiting':
      return tool ? `waiting · ${tool}` : 'waiting on you';
    case 'working':
      return tool ?? 'working…';
    case 'idle':
      return a.last_message ? `idle · ${a.last_message}` : 'idle';
  }
}
