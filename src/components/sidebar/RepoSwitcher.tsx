import { useState } from 'react';
import { Plus, X, AlertTriangle, GitBranch, ChevronDown } from 'lucide-react';
import type { Repo, Worktree, WorktreeStatus } from '../../types/ipc';

function dotState(st: WorktreeStatus | undefined): string {
  if (st?.remote_branch_gone) return 'gone';
  if (st && st.modified + st.staged > 0) return 'dirty';
  if (st) return 'clean';
  return 'idle';
}

/**
 * Repo picker pinned to the top of the sidebar: a single trigger showing the
 * active repo + branch, with a dropdown listing every repo in the task (each
 * with its git status). Selecting one scopes the whole sidebar to it. The
 * StatusBar keeps the always-visible at-a-glance status of every repo.
 */
export function RepoSwitcher({
  repos, activeRepoId, worktreeForRepo, worktreeStatus, onSelect, onAddRepo, onCloseRepo,
}: {
  repos: Repo[];
  activeRepoId: string | null;
  worktreeForRepo: (id: string) => Worktree | undefined;
  worktreeStatus: Record<string, WorktreeStatus>;
  onSelect: (id: string) => void;
  onAddRepo: () => void;
  onCloseRepo: (worktreeId: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [confirmId, setConfirmId] = useState<string | null>(null);

  const close = () => { setOpen(false); setConfirmId(null); };
  const statusFor = (repoId: string) => {
    const wt = worktreeForRepo(repoId);
    return wt ? worktreeStatus[wt.id] : undefined;
  };

  const active = repos.find((r) => r.id === activeRepoId) ?? null;

  if (!active) {
    return (
      <div className="repo-picker">
        <button className="repo-picker-trigger empty" onClick={onAddRepo}>
          <Plus size={14} strokeWidth={2} />
          <span>Add repo to task</span>
        </button>
      </div>
    );
  }

  const activeWt = worktreeForRepo(active.id);

  return (
    <div className="repo-picker">
      <button
        className="repo-picker-trigger"
        onClick={() => setOpen((o) => !o)}
        title={activeWt ? `${active.project} · ${activeWt.branch}` : active.project}
      >
        <div className="repo-picker-row-top">
          <span className={`repo-chip-dot ${dotState(statusFor(active.id))}`} />
          <span className="repo-picker-name">{active.project}</span>
          <ChevronDown size={14} strokeWidth={2} className="repo-picker-caret" />
        </div>
        {activeWt && (
          <div className="repo-picker-row-branch">
            <GitBranch size={11} strokeWidth={1.75} />
            <span className="repo-picker-branch">{activeWt.branch}</span>
          </div>
        )}
      </button>

      {open && (
        <>
          <div className="repo-picker-backdrop" onClick={close} />
          <div className="repo-picker-menu">
            {repos.map((repo) => {
              const wt = worktreeForRepo(repo.id);
              const st = wt ? worktreeStatus[wt.id] : undefined;
              const dirty = st ? st.modified + st.staged : 0;
              const ahead = st?.ahead ?? 0;
              const behind = st?.behind ?? 0;
              const gone = st?.remote_branch_gone ?? false;

              if (confirmId === repo.id) {
                return (
                  <div key={repo.id} className="repo-picker-confirm">
                    <span className="repo-picker-confirm-text">Remove <strong>{repo.project}</strong>?</span>
                    <button
                      className="repo-chip-confirm-yes"
                      onClick={() => { if (wt) onCloseRepo(wt.id); close(); }}
                    >
                      Remove
                    </button>
                    <button className="repo-chip-confirm-no" onClick={() => setConfirmId(null)}>Cancel</button>
                  </div>
                );
              }

              return (
                <button
                  key={repo.id}
                  className={`repo-picker-item ${repo.id === activeRepoId ? 'active' : ''}`}
                  onClick={() => { onSelect(repo.id); close(); }}
                >
                  <span className={`repo-chip-dot ${dotState(st)}`} />
                  <span className="repo-picker-item-name">{repo.project}</span>
                  {wt && <span className="repo-picker-item-branch">{wt.branch}</span>}
                  <span className="repo-picker-item-stats">
                    {dirty > 0 && <span className="repo-stat-modified">~{dirty}</span>}
                    {ahead > 0 && <span className="repo-stat-ahead">↑{ahead}</span>}
                    {behind > 0 && <span className="repo-stat-behind">↓{behind}</span>}
                    {gone && <AlertTriangle size={11} strokeWidth={2} style={{ color: 'var(--wb-warn)' }} />}
                  </span>
                  {wt && (
                    <span
                      className="repo-picker-close"
                      role="button"
                      tabIndex={0}
                      title="Close repo — detach from task and delete its worktree"
                      onClick={(e) => { e.stopPropagation(); setConfirmId(repo.id); }}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); setConfirmId(repo.id); }
                      }}
                    >
                      <X size={12} strokeWidth={2.25} />
                    </span>
                  )}
                </button>
              );
            })}

            <button className="repo-picker-add" onClick={() => { onAddRepo(); close(); }}>
              <Plus size={13} strokeWidth={2} />
              <span>Add repo to task…</span>
            </button>
          </div>
        </>
      )}
    </div>
  );
}
