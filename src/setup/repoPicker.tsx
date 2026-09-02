import { useCallback, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { Search, GitBranch } from 'lucide-react';
import type { Repo, MainRepo } from '../shared/ipc/ipc';
import { Highlighted } from '../shared/lib/match';
import { Combobox } from '../shared/ui/Combobox';

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
          slug: mr.slug,
          localPath: mr.local_path,
        });
        setSelectedRepos((p) => (p.some((r) => r.local_path === repo.local_path) ? p : [...p, repo]));
        opts?.onSelect?.(repo);
      } catch (e) {
        opts?.onError?.(`Could not register ${mr.slug}: ${e}`);
      } finally {
        setPending((p) => { const n = new Set(p); n.delete(mr.local_path); return n; });
      }
    },
    [pending, selectedRepos, opts],
  );

  /** Drop a selected repo by path — the selected list holds `Repo`, not `MainRepo`. */
  const deselect = useCallback(
    (localPath: string) => {
      const hit = selectedRepos.find((r) => r.local_path === localPath);
      if (!hit) return;
      setSelectedRepos((p) => p.filter((r) => r.local_path !== localPath));
      opts?.onDeselect?.(hit);
    },
    [selectedRepos, opts],
  );

  return {
    mainRepos, setMainRepos,
    selectedRepos, setSelectedRepos,
    isSelected, isPending, toggleRepo, deselect, loadRepos,
  };
}

/**
 * Search the clone pool and pick repos from the results. Selected repos are
 * listed by the caller, which owns the per-repo branch fields.
 */
export function RepoPickerSearch({
  repos, isSelected, isPending, onToggle,
}: {
  repos: MainRepo[];
  isSelected: (mr: MainRepo) => boolean;
  isPending: (mr: MainRepo) => boolean;
  onToggle: (mr: MainRepo) => void;
}) {
  return (
    <div className="repo-search">
      <Combobox
        items={repos}
        toText={(mr) => mr.slug}
        onPick={(mr) => { if (!isPending(mr)) onToggle(mr); }}
        autoFocus
        icon={Search}
        placeholder={repos.length ? `Search ${repos.length} repos…` : 'No repo in the pool'}
        disabled={repos.length === 0}
        emptyLabel="No repo matches"
        renderItem={(mr, ranges) => (
          <>
            <span className="cbx-check">
              {isPending(mr) ? '…' : isSelected(mr) ? '✓' : ''}
            </span>
            <span className="cbx-name"><Highlighted text={mr.slug} ranges={ranges} /></span>
          </>
        )}
      />
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
