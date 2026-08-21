import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  Boxes, Check, ChevronDown, Code2, Eye, FolderGit2, GitBranch, ListTodo, X,
  type LucideIcon,
} from 'lucide-react';
import { useStore, useSession, type SessionKind, type SessionState } from '../shared/store';
import { endSession } from '../shared/lib/endSession';
import { goToSession } from '../shared/lib/goToSession';
import { statusKey } from '../shared/lib/taskStatus';
import type { AgentActivity } from '../shared/ipc/ipc';

/**
 * The header context pickers: Active Session · Repo · Worktree. Each chip is
 * compact — an icon plus the id/name — and opens a plain dropdown list of the
 * choices, like the repo switcher. Repo and worktree chips show whenever the
 * session has at least one — a single-choice chip still names the current repo
 * or branch. The list is PORTALLED to the body with fixed positioning so the
 * header's `overflow: hidden` can never clip it.
 */

const KIND_ICON: Record<SessionKind, LucideIcon> = {
  task: ListTodo,
  explorer: Boxes,
  review: Eye,
};
const KIND_LABEL: Record<SessionKind, string> = {
  task: 'task',
  explorer: 'expl',
  review: 'review',
};

/** What the agent is doing, in one line. */
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

/** Chip + portalled dropdown shell shared by the three pickers. Open state lives
 *  in the store (`openPicker`) so the Alt+S/R/W shortcuts can drive it. */
function Picker({
  kind, value, icon: Icon, children, chipTitle, ariaLabel,
}: {
  kind: 'session' | 'repo' | 'worktree';
  value: React.ReactNode;
  icon: LucideIcon;
  children: React.ReactNode;
  chipTitle?: string;
  ariaLabel: string;
}) {
  const open = useStore((s) => s.openPicker === kind);
  const setOpenPicker = useStore((s) => s.setOpenPicker);
  const setOpen = (v: boolean) => setOpenPicker(v ? kind : null);
  const chipRef = useRef<HTMLButtonElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

  // Anchor the fixed-position dropdown just under the chip.
  useLayoutEffect(() => {
    if (!open) { setPos(null); return; }
    const r = chipRef.current?.getBoundingClientRect();
    if (r) setPos({ left: r.left, top: r.bottom + 6 });
  }, [open]);

  // Click-away + Escape close. The list is portalled, so check both nodes.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (chipRef.current?.contains(t) || popRef.current?.contains(t)) return;
      setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false); };
    // The window moving out from under a fixed popover would leave it stranded.
    const onMove = () => setOpen(false);
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    window.addEventListener('resize', onMove);
    window.addEventListener('blur', onMove);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
      window.removeEventListener('resize', onMove);
      window.removeEventListener('blur', onMove);
    };
  }, [open, setOpen]);

  return (
    <span className="hp">
      <button
        ref={chipRef}
        className={`hp-chip ${open ? 'open' : ''}`}
        onClick={() => setOpen(!open)}
        title={chipTitle}
        aria-label={ariaLabel}
      >
        <Icon size={13} strokeWidth={1.75} className="hp-chip-icon" />
        <span className="hp-chip-v">{value}</span>
        <ChevronDown size={11} strokeWidth={2} className="hp-chip-caret" />
      </button>
      {open && pos && createPortal(
        <div className="hp-pop" ref={popRef} style={{ left: pos.left, top: pos.top }}>
          {children}
        </div>,
        document.body,
      )}
    </span>
  );
}

function SessionRows({ onPick }: { onPick: () => void }) {
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const activeId = useStore((s) => s.activeSessionId);
  const agentActivity = useStore((s) => s.agentActivity);
  const [cursor, setCursor] = useState(() => Math.max(0, sessionOrder.indexOf(activeId ?? '')));
  const listRef = useRef<HTMLDivElement>(null);

  const rows = useMemo(
    () => sessionOrder.map((id) => sessions[id]).filter((s): s is SessionState => !!s),
    [sessionOrder, sessions],
  );

  useEffect(() => { listRef.current?.focus(); }, []);

  // Alt+S cycling switches the active session; keep the highlight on it.
  useEffect(() => {
    const i = rows.findIndex((r) => r.id === activeId);
    if (i >= 0) setCursor(i);
  }, [activeId, rows]);

  const pick = (s: SessionState) => {
    if (s.task?.short_id) goToSession(s.task.short_id);
    else useStore.getState().focusSession(s.id);
    onPick();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    const last = rows.length - 1;
    if (e.key === 'j' || e.key === 'ArrowDown') { e.preventDefault(); setCursor((c) => Math.min(c + 1, last)); }
    else if (e.key === 'k' || e.key === 'ArrowUp') { e.preventDefault(); setCursor((c) => Math.max(c - 1, 0)); }
    else if (e.key === 'Enter') { e.preventDefault(); const s = rows[cursor]; if (s) pick(s); }
  };

  if (rows.length === 0) {
    return <p className="hp-empty">No open sessions — open a task from Home.</p>;
  }

  return (
    <div className="hp-rows" tabIndex={-1} ref={listRef} onKeyDown={onKeyDown}>
      {rows.map((s, i) => {
        const Icon = KIND_ICON[s.kind] ?? Code2;
        const taskId = s.task?.short_id;
        const activity = taskId ? agentActivity[taskId] : undefined;
        const title = s.task?.title || s.title;
        const status = s.task?.status;
        return (
          <div key={s.id} className={`hp-row ${s.id === activeId ? 'active' : ''} ${i === cursor ? 'cursor' : ''}`}>
            <button className="hp-row-main" onClick={() => pick(s)} onMouseEnter={() => setCursor(i)} title={title}>
              <span className="hp-row-title">
                <Icon size={12} strokeWidth={1.75} className="hp-row-kind-icon" />
                {taskId && <span className="hp-row-id">{taskId}</span>}
                <span className="hp-row-name">{title}</span>
              </span>
              <span className="hp-row-meta">
                <span className="hp-row-kind">{KIND_LABEL[s.kind]}</span>
                {activity ? (
                  <span className={`hp-row-state ${activity.state}`}>
                    <span className={`pill-dot ${activity.state}`} />
                    {agentLine(activity)}
                  </span>
                ) : status ? (
                  <span className={`hp-row-status status-${statusKey(status)}`}>{status}</span>
                ) : null}
              </span>
            </button>
            <button
              className="hp-row-close"
              title="Close session"
              onClick={(e) => { e.stopPropagation(); endSession(s.id); }}
            >
              <X size={11} strokeWidth={2.25} />
            </button>
          </div>
        );
      })}
    </div>
  );
}

export function HeaderPickers() {
  const view = useStore((s) => s.view);
  const sessionCount = useStore((s) => s.sessionOrder.length);
  const setOpenPicker = useStore((s) => s.setOpenPicker);

  const sessionId = useSession((s) => s.id);
  const sessionKind = useSession((s) => s.kind);
  const sessionTitle = useSession((s) => s.title);
  const sessionShortId = useSession((s) => s.task?.short_id ?? null);
  const repos = useSession((s) => s.repos);
  const worktrees = useSession((s) => s.worktrees);
  const activeRepoId = useSession((s) => s.activeRepoId);
  const activeWorktreeId = useSession((s) => s.activeWorktreeId);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);
  const setActiveWorktreeId = useSession((s) => s.setActiveWorktreeId);

  const inWorkspace = view === 'workspace' && !!sessionId;
  const activeRepo = repos.find((r) => r.id === activeRepoId) ?? null;
  const repoWorktrees = worktrees.filter((w) => w.repo_id === activeRepoId);
  const activeWt = worktrees.find((w) => w.id === activeWorktreeId) ?? null;

  // The session chip is useful everywhere (it also closes sessions); the
  // repo/worktree chips are workspace context and hide with it.
  if (sessionCount === 0 && !inWorkspace) return null;

  // Chip shows icon + id (session), name (repo) or branch (worktree) — nothing more.
  const sessionValue = inWorkspace
    ? <span className="hp-chip-id">{sessionShortId ?? sessionTitle}</span>
    : <span className="hp-chip-name">{sessionCount} open</span>;

  return (
    <span className="header-pickers">
      <Picker
        kind="session"
        ariaLabel="Sessions"
        value={sessionValue}
        icon={inWorkspace ? (KIND_ICON[sessionKind] ?? Code2) : Code2}
        chipTitle="Switch or close sessions (Alt+S)"
      >
        <SessionRows onPick={() => setOpenPicker(null)} />
      </Picker>

      {inWorkspace && repos.length > 0 && (
        <Picker
          kind="repo"
          ariaLabel="Repository"
          value={(activeRepo ?? repos[0])?.project ?? '—'}
          icon={FolderGit2}
          chipTitle="The repo git actions target (Alt+R)"
        >
          <div className="hp-rows">
            {repos.map((r) => (
              <button
                key={r.id}
                className={`hp-row hp-row-simple ${r.id === activeRepoId ? 'active' : ''}`}
                onClick={() => { setActiveRepoId(r.id); setOpenPicker(null); }}
              >
                <span className="hp-tick">{r.id === activeRepoId && <Check size={12} strokeWidth={2.5} />}</span>
                <FolderGit2 size={12} strokeWidth={1.75} />
                <span className="hp-row-title">{r.project}</span>
                <span className="hp-row-kind">{r.host}</span>
              </button>
            ))}
          </div>
        </Picker>
      )}

      {inWorkspace && repoWorktrees.length > 0 && (
        <Picker
          kind="worktree"
          ariaLabel="Worktree"
          value={(activeWt ?? repoWorktrees[0])?.branch ?? '—'}
          icon={GitBranch}
          chipTitle="The worktree git actions target (Alt+W)"
        >
          <div className="hp-rows">
            {repoWorktrees.map((w) => (
              <button
                key={w.id}
                className={`hp-row hp-row-simple ${w.id === activeWorktreeId ? 'active' : ''}`}
                onClick={() => { setActiveWorktreeId(w.id); setOpenPicker(null); }}
              >
                <span className="hp-tick">{w.id === activeWorktreeId && <Check size={12} strokeWidth={2.5} />}</span>
                <GitBranch size={12} strokeWidth={1.75} />
                <span className="hp-row-title">{w.branch}</span>
              </button>
            ))}
          </div>
        </Picker>
      )}
    </span>
  );
}
