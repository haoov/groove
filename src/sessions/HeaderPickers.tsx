import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Boxes, ChevronDown, Code2, Eye, FolderGit2, GitBranch, ListTodo, X, type LucideIcon,
} from 'lucide-react';
import { useStore, useSession, type SessionKind, type SessionState } from '../shared/store';
import { endSession } from '../shared/lib/endSession';
import { goToSession } from '../shared/lib/goToSession';
import { statusKey } from '../shared/lib/taskStatus';
import type { AgentActivity } from '../shared/ipc/ipc';

/**
 * The header context pickers: Active Session · Repo · Worktree (the mockup's
 * chip pickers — they replaced the session dock). The session chip switches and
 * closes sessions; the repo/worktree chips scope the sidebar, commit panel and
 * diff to one checkout. Repo/worktree chips only render when there is a choice.
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

/** Chip + popover shell shared by the three pickers. */
function Picker({
  label, value, icon: Icon, open, setOpen, children, chipTitle,
}: {
  label: string;
  value: string;
  icon: LucideIcon;
  open: boolean;
  setOpen: (v: boolean) => void;
  children: React.ReactNode;
  chipTitle?: string;
}) {
  const popRef = useRef<HTMLDivElement>(null);

  // Click-away + Escape close, like every other popover surface.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!popRef.current?.parentElement?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false); };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open, setOpen]);

  return (
    <span className="hp">
      <button className={`hp-chip ${open ? 'open' : ''}`} onClick={() => setOpen(!open)} title={chipTitle}>
        <span className="hp-chip-k">{label}</span>
        <Icon size={12} strokeWidth={1.75} className="hp-chip-icon" />
        <span className="hp-chip-v">{value}</span>
        <ChevronDown size={11} strokeWidth={2} className="hp-chip-caret" />
      </button>
      {open && <div className="hp-pop" ref={popRef}>{children}</div>}
    </span>
  );
}

function SessionRows({ onPick }: { onPick: () => void }) {
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const activeId = useStore((s) => s.activeSessionId);
  const agentActivity = useStore((s) => s.agentActivity);
  const [cursor, setCursor] = useState(() =>
    Math.max(0, sessionOrder.indexOf(activeId ?? '')));
  const listRef = useRef<HTMLDivElement>(null);

  const rows = useMemo(
    () => sessionOrder.map((id) => sessions[id]).filter((s): s is SessionState => !!s),
    [sessionOrder, sessions],
  );

  useEffect(() => { listRef.current?.focus(); }, []);

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
              <span className="hp-row-title">{title}</span>
              <span className="hp-row-meta">
                <Icon size={11} strokeWidth={1.75} />
                {taskId && <span className="hp-row-id">{taskId}</span>}
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
  // Alt+S opens the session picker from anywhere (the dock's old shortcut).
  const sessionPickerOpen = useStore((s) => s.sessionPickerOpen);
  const setSessionPickerOpen = useStore((s) => s.setSessionPickerOpen);
  const [repoOpen, setRepoOpen] = useState(false);
  const [wtOpen, setWtOpen] = useState(false);

  const session = useSession((s) => (s.id ? { id: s.id, kind: s.kind, title: s.title } : null));
  const repos = useSession((s) => s.repos);
  const worktrees = useSession((s) => s.worktrees);
  const activeRepoId = useSession((s) => s.activeRepoId);
  const activeWorktreeId = useSession((s) => s.activeWorktreeId);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);
  const setActiveWorktreeId = useSession((s) => s.setActiveWorktreeId);

  const inWorkspace = view === 'workspace' && !!session;
  const activeRepo = repos.find((r) => r.id === activeRepoId) ?? null;
  const repoWorktrees = worktrees.filter((w) => w.repo_id === activeRepoId);
  const activeWt = worktrees.find((w) => w.id === activeWorktreeId) ?? null;

  // The session chip is useful everywhere (it also closes sessions); the
  // repo/worktree chips are workspace context and hide with it.
  if (sessionCount === 0 && !inWorkspace) return null;

  return (
    <span className="header-pickers">
      <Picker
        label="session"
        value={inWorkspace ? session!.title : `${sessionCount} open`}
        icon={inWorkspace ? (KIND_ICON[session!.kind] ?? Code2) : Code2}
        open={sessionPickerOpen}
        setOpen={setSessionPickerOpen}
        chipTitle="Switch or close sessions (Alt+S)"
      >
        <SessionRows onPick={() => setSessionPickerOpen(false)} />
      </Picker>

      {inWorkspace && repos.length > 1 && (
        <Picker
          label="repo"
          value={activeRepo?.project ?? '—'}
          icon={FolderGit2}
          open={repoOpen}
          setOpen={setRepoOpen}
          chipTitle="The repo git actions target"
        >
          <div className="hp-rows">
            {repos.map((r) => (
              <button
                key={r.id}
                className={`hp-row hp-row-simple ${r.id === activeRepoId ? 'active' : ''}`}
                onClick={() => { setActiveRepoId(r.id); setRepoOpen(false); }}
              >
                <FolderGit2 size={11} strokeWidth={1.75} />
                <span className="hp-row-title">{r.project}</span>
                <span className="hp-row-kind">{r.host}</span>
              </button>
            ))}
          </div>
        </Picker>
      )}

      {inWorkspace && repoWorktrees.length > 1 && (
        <Picker
          label="worktree"
          value={activeWt?.branch ?? '—'}
          icon={GitBranch}
          open={wtOpen}
          setOpen={setWtOpen}
          chipTitle="The worktree git actions target (this repo has several)"
        >
          <div className="hp-rows">
            {repoWorktrees.map((w) => (
              <button
                key={w.id}
                className={`hp-row hp-row-simple ${w.id === activeWorktreeId ? 'active' : ''}`}
                onClick={() => { setActiveWorktreeId(w.id); setWtOpen(false); }}
              >
                <GitBranch size={11} strokeWidth={1.75} />
                <span className="hp-row-title">{w.branch}</span>
              </button>
            ))}
          </div>
        </Picker>
      )}
    </span>
  );
}
