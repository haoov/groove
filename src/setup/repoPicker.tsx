import { useCallback, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Search, GitBranch } from 'lucide-react';
import type { Repo, MainRepo } from '../shared/ipc/ipc';
import { matchRanges, Highlighted } from '../shared/lib/match';

/**
 * Shared repo selection for the task-open wizard and the add-repo modal.
 * The repo pool is whatever lives under `<worktree_root>/main/**` (scanned by
 * the backend) — no config list. New repos arrive by cloning a git URL into
 * the pool via `CloneRepoForm`.
 */
export function useRepoPicker(opts?: {
  onSelect?: (repo: Repo) => void;
  onDeselect?: (repo: Repo) => void;
  onError?: (msg: string) => void;
}) {
  const [mainRepos, setMainRepos] = useState<MainRepo[]>([]);
  const [selectedRepos, setSelectedRepos] = useState<Repo[]>([]);
  // local_path values with a register_repo call in flight.
  const [pending, setPending] = useState<Set<string>>(new Set());

  const loadRepos = useCallback(() => {
    invoke<MainRepo[]>('list_main_repos')
      .then(setMainRepos)
      .catch((e) => opts?.onError?.(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const isSelected = useCallback(
    (mr: MainRepo) => selectedRepos.some((r) => r.local_path === mr.local_path),
    [selectedRepos],
  );
  const isPending = useCallback((mr: MainRepo) => pending.has(mr.local_path), [pending]);

  const toggleRepo = useCallback(
    async (mr: MainRepo) => {
      if (pending.has(mr.local_path)) return; // guard against double-click double-register
      const already = selectedRepos.find((r) => r.local_path === mr.local_path);
      if (already) {
        setSelectedRepos((p) => p.filter((r) => r.local_path !== mr.local_path));
        opts?.onDeselect?.(already);
        return;
      }
      setPending((p) => { const n = new Set(p); n.add(mr.local_path); return n; });
      try {
        const repo = await invoke<Repo>('register_repo', {
          localPath: mr.local_path,
          remoteUrl: mr.url,
        });
        setSelectedRepos((p) => (p.some((r) => r.local_path === repo.local_path) ? p : [...p, repo]));
        opts?.onSelect?.(repo);
      } catch (e) {
        opts?.onError?.(`Could not register ${mr.url}: ${e}`);
      } finally {
        setPending((p) => { const n = new Set(p); n.delete(mr.local_path); return n; });
      }
    },
    [pending, selectedRepos, opts],
  );

  return {
    mainRepos, setMainRepos,
    selectedRepos, setSelectedRepos,
    isSelected, isPending, toggleRepo, loadRepos,
  };
}

/** Fuzzy-searchable list of the pooled repos (rows disabled while registering). */
export function RepoPickerList({
  repos, isSelected, isPending, onToggle, onConfirm,
}: {
  repos: MainRepo[];
  isSelected: (mr: MainRepo) => boolean;
  isPending: (mr: MainRepo) => boolean;
  onToggle: (mr: MainRepo) => void;
  /** Enter in the filter — submit whatever is selected. */
  onConfirm?: () => void;
}) {
  const [query, setQuery] = useState('');
  const [cursor, setCursor] = useState(0);
  const filtered = useMemo(() => {
    const q = query.trim();
    if (!q) return repos.map((r) => ({ repo: r, ranges: [] as [number, number][] }));
    return repos
      .map((r) => ({ repo: r, ranges: matchRanges(q, r.slug) }))
      .filter((x): x is { repo: MainRepo; ranges: [number, number][] } => x.ranges !== null);
  }, [repos, query]);

  // Filtering shrinks the list under the cursor.
  const at = Math.min(cursor, Math.max(0, filtered.length - 1));

  /**
   * Ctrl+J/K move, Ctrl+Tab toggles, Enter submits — the whole wizard without
   * the mouse. Ctrl rather than bare j/k because focus is in a text field.
   */
  const onKeyDown = (e: React.KeyboardEvent) => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && (e.key === 'j' || e.key === 'k')) {
      e.preventDefault();
      if (!filtered.length) return;
      const d = e.key === 'j' ? 1 : -1;
      setCursor((c) => (Math.min(c, filtered.length - 1) + d + filtered.length) % filtered.length);
    } else if (ctrl && e.key === 'Tab') {
      e.preventDefault();
      const hit = filtered[at];
      if (hit) onToggle(hit.repo);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      onConfirm?.();
    }
  };

  return (
    <div className="wizard-repo-pool">
      <div className="wizard-repo-search">
        <Search size={13} strokeWidth={1.75} className="file-search-icon" />
        <input
          className="wizard-repo-search-input"
          placeholder="Filter repos…"
          value={query}
          onChange={(e) => { setQuery(e.target.value); setCursor(0); }}
          onKeyDown={onKeyDown}
          // Ctrl+J/K would otherwise be eaten by the global capture-phase keymap.
          data-repo-picker="1"
          autoFocus
        />
        <span className="wizard-repo-count">{filtered.length}/{repos.length}</span>
      </div>
      <div className="wizard-repo-list">
        {filtered.length === 0 && (
          <p className="wizard-empty">No repo matches “{query}”.</p>
        )}
        {filtered.map(({ repo: mr, ranges }, i) => {
          const sel = isSelected(mr);
          const busy = isPending(mr);
          return (
            <button
              key={mr.local_path}
              className={`wizard-repo-item ${sel ? 'selected' : ''} ${i === at ? 'cursor' : ''}`}
              onMouseEnter={() => setCursor(i)}
              onClick={() => onToggle(mr)}
              disabled={busy}
              title={mr.url}
            >
              <span className="wizard-repo-check">{busy ? '…' : sel ? '✓' : ' '}</span>
              <span className="wizard-repo-name">
                {ranges.length ? <Highlighted text={mr.slug} ranges={ranges} /> : mr.slug}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

/** Clone a new repo by URL — it lands at main/<host>/<group>/<project>. */
export function CloneRepoForm({ onCloned }: { onCloned: (repo: MainRepo) => void }) {
  const [showForm, setShowForm] = useState(false);
  const [url, setUrl] = useState('');
  const [cloning, setCloning] = useState(false);
  const [error, setError] = useState('');

  const clone = async () => {
    const u = url.trim();
    if (!u || cloning) return;
    setCloning(true);
    setError('');
    try {
      const repo = await invoke<MainRepo>('clone_repo', { url: u });
      onCloned(repo);
      setUrl('');
      setShowForm(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setCloning(false);
    }
  };

  if (!showForm) {
    return (
      <button className="wizard-add-toggle" onClick={() => setShowForm(true)}>
        <GitBranch size={12} strokeWidth={1.75} style={{ marginRight: 5, verticalAlign: 'middle' }} />
        Clone a new repo…
      </button>
    );
  }

  return (
    <div className="wizard-add-form">
      <p className="wizard-add-hint">
        Paste a git URL — it clones under <code>main/&lt;host&gt;/&lt;group&gt;/&lt;project&gt;</code> and joins the list.
      </p>
      <div className="wizard-input-row">
        <input
          className="wizard-input"
          placeholder="git@gitlab.com:group/project.git"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && clone()}
          disabled={cloning}
          autoFocus
        />
        <button className="btn-secondary" onClick={clone} disabled={cloning || !url.trim()}>
          {cloning ? 'Cloning…' : 'Clone'}
        </button>
        <button
          className="btn-ghost"
          onClick={() => { setShowForm(false); setError(''); setUrl(''); }}
          disabled={cloning}
        >
          Cancel
        </button>
      </div>
      {error && <div className="wizard-error">{error}</div>}
    </div>
  );
}
