import { useEffect, useRef, useState } from 'react';
import { GitBranch } from 'lucide-react';
import { useStore, useSession } from '../store';

/**
 * Alt+R: pick which repo of the session you are working in.
 *
 * The active repo decides where the git commands act (see CommandPalette, which
 * used to take whichever worktree came first), so on a multi-repo task switching it
 * needs to be one keystroke rather than a trip to the status bar.
 *
 * Ctrl+J/K to move — matching the repo-add wizard, and leaving plain j/k alone
 * because this can open over a vim-navigable surface.
 */
export function RepoSwitcher() {
  const open = useStore((s) => s.repoSwitcherOpen);
  const setOpen = useStore((s) => s.setRepoSwitcherOpen);
  const repos = useSession((s) => s.activeRepos);
  const worktrees = useSession((s) => s.activeWorktrees);
  const status = useSession((s) => s.worktreeStatus);
  const activeRepoId = useSession((s) => s.activeRepoId);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);

  const [cursor, setCursor] = useState(0);
  const boxRef = useRef<HTMLDivElement>(null);

  // Start on the repo you are in, so Enter is a no-op rather than a jump.
  useEffect(() => {
    if (!open) return;
    const at = repos.findIndex((r) => r.id === activeRepoId);
    setCursor(at >= 0 ? at : 0);
    boxRef.current?.focus();
  }, [open]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      const down = (ctrl && e.key === 'j') || e.key === 'ArrowDown';
      const up = (ctrl && e.key === 'k') || e.key === 'ArrowUp';
      if (down || up) {
        e.preventDefault();
        setCursor((c) => (repos.length ? (c + (down ? 1 : -1) + repos.length) % repos.length : 0));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const pick = repos[cursor];
        if (pick) setActiveRepoId(pick.id);
        setOpen(false);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        setOpen(false);
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [open, repos, cursor, setActiveRepoId, setOpen]);

  if (!open) return null;

  return (
    <div className="palette-overlay" onClick={() => setOpen(false)}>
      <div className="repo-switcher" ref={boxRef} tabIndex={-1} onClick={(e) => e.stopPropagation()}>
        <div className="repo-switcher-head">Switch repo</div>
        {repos.length === 0 ? (
          <p className="palette-empty">No repos on this session.</p>
        ) : (
          <div className="repo-switcher-list">
            {repos.map((r, i) => {
              const wt = worktrees.find((w) => w.repo_id === r.id);
              const st = wt ? status[wt.id] : undefined;
              const dirty = (st?.modified ?? 0) + (st?.staged ?? 0);
              return (
                <button
                  key={r.id}
                  className={`repo-switcher-row ${i === cursor ? 'cursor' : ''} ${r.id === activeRepoId ? 'active' : ''}`}
                  onMouseEnter={() => setCursor(i)}
                  onClick={() => { setActiveRepoId(r.id); setOpen(false); }}
                >
                  <span className="repo-switcher-name">{r.project}</span>
                  <span className="repo-switcher-meta">
                    <GitBranch size={10} strokeWidth={1.75} />
                    <span className="repo-switcher-branch">{wt?.branch ?? 'not provisioned'}</span>
                    {dirty > 0 && <span className="repo-switcher-dirty">{dirty}</span>}
                  </span>
                </button>
              );
            })}
          </div>
        )}
        <div className="repo-switcher-foot">
          <span className="palette-footer-hints">
            <kbd>ctrl+j</kbd>/<kbd>ctrl+k</kbd> move <kbd>⏎</kbd> switch <kbd>esc</kbd> close
          </span>
        </div>
      </div>
    </div>
  );
}
