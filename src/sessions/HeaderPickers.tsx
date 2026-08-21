import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Boxes, Check, ChevronDown, Code2, Eye, FolderGit2, GitBranch, ListTodo, Search, X,
  type LucideIcon,
} from 'lucide-react';
import { useStore, useSession, type SessionKind, type SessionState } from '../shared/store';
import { endSession } from '../shared/lib/endSession';
import { goToSession } from '../shared/lib/goToSession';
import { statusKey } from '../shared/lib/taskStatus';
import type { AgentActivity } from '../shared/ipc/ipc';

/**
 * The header context pickers: Active Session · Repo · Worktree. Modelled on Zed's
 * title-bar project/branch switcher — quiet chips (icon + value, no boxy label)
 * that open a filterable command-list. The session chip switches and closes
 * sessions and always shows the id, so which session is active is never in doubt.
 * The repo/worktree chips scope the sidebar, commit panel and diff, and only
 * appear when there is more than one to pick.
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
  value: React.ReactNode;
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
      <button
        className={`hp-chip ${open ? 'open' : ''}`}
        onClick={() => setOpen(!open)}
        title={chipTitle}
        aria-label={label}
      >
        <Icon size={13} strokeWidth={1.75} className="hp-chip-icon" />
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
  const [query, setQuery] = useState('');
  const [cursor, setCursor] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const rows = useMemo(
    () => sessionOrder.map((id) => sessions[id]).filter((s): s is SessionState => !!s),
    [sessionOrder, sessions],
  );

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((s) => {
      const hay = `${s.task?.short_id ?? ''} ${s.task?.title ?? s.title} ${s.kind}`.toLowerCase();
      return hay.includes(q);
    });
  }, [rows, query]);

  // Focus the filter on open; keep the cursor in range as the list narrows.
  useEffect(() => { inputRef.current?.focus(); }, []);
  useEffect(() => {
    setCursor((c) => Math.min(Math.max(0, c), Math.max(0, filtered.length - 1)));
  }, [filtered.length]);

  const pick = (s: SessionState) => {
    if (s.task?.short_id) goToSession(s.task.short_id);
    else useStore.getState().focusSession(s.id);
    onPick();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    const last = filtered.length - 1;
    if (e.key === 'ArrowDown') { e.preventDefault(); setCursor((c) => Math.min(c + 1, last)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setCursor((c) => Math.max(c - 1, 0)); }
    else if (e.key === 'Enter') { e.preventDefault(); const s = filtered[cursor]; if (s) pick(s); }
  };

  return (
    <>
      <div className="hp-search">
        <Search size={13} strokeWidth={1.75} />
        <input
          ref={inputRef}
          value={query}
          placeholder="Search sessions…"
          spellCheck={false}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
        />
      </div>
      {filtered.length === 0 ? (
        <p className="hp-empty">
          {rows.length === 0 ? 'No open sessions — open a task from Home.' : 'No sessions match.'}
        </p>
      ) : (
        <div className="hp-rows">
          {filtered.map((s, i) => {
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
      )}
    </>
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

  const sessionValue = inWorkspace ? (
    <>
      {sessionShortId && <span className="hp-chip-id">{sessionShortId}</span>}
      <span className="hp-chip-name">{sessionTitle}</span>
    </>
  ) : (
    <span className="hp-chip-name">{sessionCount} open</span>
  );

  return (
    <span className="header-pickers">
      <Picker
        label="Sessions"
        value={sessionValue}
        icon={inWorkspace ? (KIND_ICON[sessionKind] ?? Code2) : Code2}
        open={sessionPickerOpen}
        setOpen={setSessionPickerOpen}
        chipTitle="Switch or close sessions (Alt+S)"
      >
        <SessionRows onPick={() => setSessionPickerOpen(false)} />
      </Picker>

      {inWorkspace && repos.length > 1 && (
        <Picker
          label="Repository"
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
                <span className="hp-tick">{r.id === activeRepoId && <Check size={12} strokeWidth={2.5} />}</span>
                <FolderGit2 size={12} strokeWidth={1.75} />
                <span className="hp-row-title">{r.project}</span>
                <span className="hp-row-kind">{r.host}</span>
              </button>
            ))}
          </div>
        </Picker>
      )}

      {inWorkspace && repoWorktrees.length > 1 && (
        <Picker
          label="Worktree"
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
