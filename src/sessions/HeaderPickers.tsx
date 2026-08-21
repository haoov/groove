import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  Boxes, Check, ChevronDown, Code2, Eye, FolderGit2, GitBranch, ListTodo, Plus, X,
  type LucideIcon,
} from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore, useSession, type SessionKind, type SessionState } from '../shared/store';
import { endSession } from '../shared/lib/endSession';
import { goToSession } from '../shared/lib/goToSession';
import { statusKey } from '../shared/lib/taskStatus';
import type { AgentActivity } from '../shared/ipc/ipc';

/**
 * The header context pickers: Active Session · Repo · Worktree. Each chip is
 * compact — icon + id/name — and opens a plain dropdown list of the choices.
 * Alt+S/R/W open the list and move the highlight; Enter (or a click) commits,
 * Esc cancels — nothing switches until you commit. The list is PORTALLED to the
 * body with fixed positioning so the header's overflow can never clip it.
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

/** The short id for a session: the MR number for reviews (their short_id is long
 *  and unhelpful), else the task short_id. */
function sessionIdLabel(s: SessionState): string | null {
  if (s.kind === 'review') {
    const mr = s.mrs?.[0];
    if (mr) return `${mr.platform === 'github' ? '#' : '!'}${mr.remote_id}`;
  }
  return s.task?.short_id ?? null;
}

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

/** Keyboard-navigable list state, shared by the three dropdowns. The highlight
 *  lives in the store so Alt+S/R/W can drive it; Enter commits the highlighted. */
function usePickerList(count: number, initialIndex: number, onEnter: (i: number) => void) {
  const cursor = useStore((s) => s.pickerCursor);
  const setCursor = useStore((s) => s.setPickerCursor);
  const ref = useRef<HTMLDivElement>(null);

  // On open, highlight the current item and take focus for the arrow keys.
  useEffect(() => {
    setCursor(initialIndex >= 0 ? initialIndex : 0);
    ref.current?.focus();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (count === 0) return;
    if (e.key === 'ArrowDown' || e.key === 'j') { e.preventDefault(); setCursor((cursor + 1) % count); }
    else if (e.key === 'ArrowUp' || e.key === 'k') { e.preventDefault(); setCursor((cursor - 1 + count) % count); }
    else if (e.key === 'Enter') { e.preventDefault(); onEnter(cursor); }
  };

  return { cursor, setCursor, ref, onKeyDown };
}

/** Chip + portalled dropdown shell. Open state lives in the store (`openPicker`)
 *  so the Alt+S/R/W shortcuts can drive it. */
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

  useLayoutEffect(() => {
    if (!open) { setPos(null); return; }
    const r = chipRef.current?.getBoundingClientRect();
    if (r) setPos({ left: r.left, top: r.bottom + 6 });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (chipRef.current?.contains(t) || popRef.current?.contains(t)) return;
      setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false); };
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

function SessionRows({ onClose }: { onClose: () => void }) {
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const activeId = useStore((s) => s.activeSessionId);
  const agentActivity = useStore((s) => s.agentActivity);

  const rows = useMemo(
    () => sessionOrder.map((id) => sessions[id]).filter((s): s is SessionState => !!s),
    [sessionOrder, sessions],
  );

  const pick = (s: SessionState) => {
    if (s.task?.short_id) goToSession(s.task.short_id);
    else useStore.getState().focusSession(s.id);
    onClose();
  };

  const activeIdx = rows.findIndex((r) => r.id === activeId);
  const { cursor, setCursor, ref, onKeyDown } = usePickerList(
    rows.length, activeIdx, (i) => { const s = rows[i]; if (s) pick(s); },
  );

  if (rows.length === 0) {
    return <p className="hp-empty">No open sessions — open a task from Home.</p>;
  }

  return (
    <div className="hp-rows" tabIndex={-1} ref={ref} onKeyDown={onKeyDown}>
      {rows.map((s, i) => {
        const Icon = KIND_ICON[s.kind] ?? Code2;
        const taskId = s.task?.short_id;
        const activity = taskId ? agentActivity[taskId] : undefined;
        const idLabel = sessionIdLabel(s);
        const title = s.task?.title || s.title;
        const status = s.task?.status;
        return (
          <div key={s.id} className={`hp-row ${s.id === activeId ? 'active' : ''} ${i === cursor ? 'cursor' : ''}`}>
            <button className="hp-row-main" onClick={() => pick(s)} onMouseEnter={() => setCursor(i)} title={title}>
              <span className="hp-row-title">
                <Icon size={12} strokeWidth={1.75} className="hp-row-kind-icon" />
                {idLabel && <span className="hp-row-id">{idLabel}</span>}
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

interface RepoLite { id: string; project: string; host: string }
interface WtLite { id: string; branch: string; repo_id: string }

function RepoRows({
  repos, worktrees, activeRepoId, onSelect, onClose, onAddRepo,
}: {
  repos: RepoLite[];
  worktrees: WtLite[];
  activeRepoId: string | null;
  onSelect: (id: string) => void;
  onClose: () => void;
  onAddRepo: () => void;
}) {
  const setLastError = useStore((s) => s.setLastError);
  const [confirmId, setConfirmId] = useState<string | null>(null);

  const select = (r: RepoLite) => { onSelect(r.id); onClose(); };
  // force:true so closing succeeds on a dirty worktree — the confirm step is the
  // guard, matching the old sidebar repo switcher.
  const closeRepo = async (worktreeId: string) => {
    try { await invoke('close_worktree', { worktreeId, force: true }); }
    catch (e) { setLastError(String(e)); }
    onClose();
  };

  const activeIdx = repos.findIndex((r) => r.id === activeRepoId);
  const { cursor, setCursor, ref, onKeyDown } = usePickerList(
    repos.length, activeIdx, (i) => { const r = repos[i]; if (r) select(r); },
  );

  return (
    <div className="hp-rows" tabIndex={-1} ref={ref} onKeyDown={onKeyDown}>
      {repos.map((r, i) => {
        const wt = worktrees.find((w) => w.repo_id === r.id);
        if (confirmId === r.id) {
          return (
            <div key={r.id} className="hp-row hp-confirm">
              <span className="hp-confirm-text">Remove <strong>{r.project}</strong>?</span>
              <button className="hp-confirm-yes" onClick={() => { if (wt) closeRepo(wt.id); }}>Remove</button>
              <button className="hp-confirm-no" onClick={() => setConfirmId(null)}>Cancel</button>
            </div>
          );
        }
        return (
          <div key={r.id} className={`hp-row ${r.id === activeRepoId ? 'active' : ''} ${i === cursor ? 'cursor' : ''}`}>
            <button
              className="hp-row-simple hp-row-grow"
              onClick={() => select(r)}
              onMouseEnter={() => setCursor(i)}
            >
              <span className="hp-tick">{r.id === activeRepoId && <Check size={12} strokeWidth={2.5} />}</span>
              <FolderGit2 size={12} strokeWidth={1.75} />
              <span className="hp-row-title">{r.project}</span>
              <span className="hp-row-kind">{r.host}</span>
            </button>
            {wt && (
              <button
                className="hp-row-close"
                title="Close repo — detach from the task and delete its worktree"
                onClick={(e) => { e.stopPropagation(); setConfirmId(r.id); }}
              >
                <X size={11} strokeWidth={2.25} />
              </button>
            )}
          </div>
        );
      })}
      <button className="hp-row hp-row-add" onClick={() => { onAddRepo(); onClose(); }}>
        <span className="hp-tick"><Plus size={12} strokeWidth={2} /></span>
        Add repo to task…
      </button>
    </div>
  );
}

function WorktreeRows({
  worktrees, activeWorktreeId, onSelect, onClose,
}: {
  worktrees: WtLite[];
  activeWorktreeId: string | null;
  onSelect: (id: string) => void;
  onClose: () => void;
}) {
  const select = (w: WtLite) => { onSelect(w.id); onClose(); };
  const activeIdx = worktrees.findIndex((w) => w.id === activeWorktreeId);
  const { cursor, setCursor, ref, onKeyDown } = usePickerList(
    worktrees.length, activeIdx, (i) => { const w = worktrees[i]; if (w) select(w); },
  );

  return (
    <div className="hp-rows" tabIndex={-1} ref={ref} onKeyDown={onKeyDown}>
      {worktrees.map((w, i) => (
        <button
          key={w.id}
          className={`hp-row hp-row-simple ${w.id === activeWorktreeId ? 'active' : ''} ${i === cursor ? 'cursor' : ''}`}
          onClick={() => select(w)}
          onMouseEnter={() => setCursor(i)}
        >
          <span className="hp-tick">{w.id === activeWorktreeId && <Check size={12} strokeWidth={2.5} />}</span>
          <GitBranch size={12} strokeWidth={1.75} />
          <span className="hp-row-title">{w.branch}</span>
        </button>
      ))}
    </div>
  );
}

export function HeaderPickers() {
  const view = useStore((s) => s.view);
  const sessionCount = useStore((s) => s.sessionOrder.length);
  const setOpenPicker = useStore((s) => s.setOpenPicker);
  const setAddRepoOpen = useStore((s) => s.setAddRepoOpen);

  const sessionId = useSession((s) => s.id);
  const sessionKind = useSession((s) => s.kind);
  const sessionTitle = useSession((s) => s.title);
  const sessionShortId = useSession((s) => s.task?.short_id ?? null);
  const sessionMrs = useSession((s) => s.mrs);
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
  const close = () => setOpenPicker(null);

  // The session chip is useful everywhere (it also closes sessions); the
  // repo/worktree chips are workspace context and hide with it.
  if (sessionCount === 0 && !inWorkspace) return null;

  // Reviews get the MR number, not their long title/short_id.
  const reviewNum = sessionKind === 'review' && sessionMrs[0]
    ? `${sessionMrs[0].platform === 'github' ? '#' : '!'}${sessionMrs[0].remote_id}`
    : null;
  const sessionValue = inWorkspace
    ? <span className="hp-chip-id">{reviewNum ?? sessionShortId ?? sessionTitle}</span>
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
        <SessionRows onClose={close} />
      </Picker>

      {inWorkspace && repos.length > 0 && (
        <Picker
          kind="repo"
          ariaLabel="Repository"
          value={<span className="hp-chip-name">{(activeRepo ?? repos[0])?.project ?? '—'}</span>}
          icon={FolderGit2}
          chipTitle="The repo git actions target (Alt+R)"
        >
          <RepoRows
            repos={repos}
            worktrees={worktrees}
            activeRepoId={activeRepoId}
            onSelect={setActiveRepoId}
            onClose={close}
            onAddRepo={() => setAddRepoOpen(true)}
          />
        </Picker>
      )}

      {inWorkspace && repoWorktrees.length > 0 && (
        <Picker
          kind="worktree"
          ariaLabel="Worktree"
          value={<span className="hp-chip-name">{(activeWt ?? repoWorktrees[0])?.branch ?? '—'}</span>}
          icon={GitBranch}
          chipTitle="The worktree git actions target (Alt+W)"
        >
          <WorktreeRows
            worktrees={repoWorktrees}
            activeWorktreeId={activeWorktreeId}
            onSelect={setActiveWorktreeId}
            onClose={close}
          />
        </Picker>
      )}
    </span>
  );
}
