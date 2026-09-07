import { useState, useEffect } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useSession, useStore } from '../shared/store';
import { useRepoPicker, RepoPickerSearch, CloneRepoForm } from './repoPicker';
import { BranchPicker, useOriginBranches } from './branchPicker';
import type { Repo } from '../shared/ipc/ipc';
import { errorText } from '../shared/lib/appError';

/** One repo's row: the branch to create, and the base it cuts from. */
function RepoBranchRow({
  repo, branch, onBranch, target, onTarget, defaultBranch, onRemove,
}: {
  repo: Repo;
  branch: string;
  onBranch: (v: string) => void;
  target: string;
  onTarget: (v: string) => void;
  defaultBranch: string;
  onRemove: () => void;
}) {
  const origin = useOriginBranches(repo.id);
  return (
    <div className="wizard-branch-item stacked">
      <div className="wizard-branch-head">
        <span className="wizard-branch-repo" title={repo.local_path}>{repo.project}</span>
        <button type="button" className="wizard-branch-remove" title="Remove" onClick={onRemove}>×</button>
      </div>
      <div className="wizard-branch-fields">
        <label className="wizard-branch-field">
          <span className="wizard-branch-field-label">Base</span>
          <BranchPicker state={origin} value={target} onChange={onTarget} />
        </label>
        <label className="wizard-branch-field">
          <span className="wizard-branch-field-label">Branch</span>
          <input
            className="wizard-input"
            placeholder={defaultBranch}
            value={branch}
            onChange={(e) => onBranch(e.target.value)}
          />
        </label>
      </div>
    </div>
  );
}

/** Adds repos to the open task. Existing worktrees are left untouched. */
export function AddRepoModal({ onClose }: { onClose: () => void }) {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const isExplorer = useSession((s) => s.kind === 'explorer');
  const notify = useStore((s) => s.notify);
  // The branch convention lives in the backend; ask for it, never rebuild it here.
  const [defaultBranch, setDefaultBranch] = useState((activeTask?.short_id ?? '').toLowerCase());
  useEffect(() => {
    const shortId = activeTask?.short_id;
    if (!shortId) return;
    invoke<string>('default_branch_for_session', { shortId })
      .then((b) => { if (b.trim()) setDefaultBranch(b.trim()); })
      .catch(() => { /* keep the short-id fallback */ });
  }, [activeTask?.short_id]);

  // repo.id → branch name.
  const [branchByRepo, setBranchByRepo] = useState<Record<string, string>>({});
  // repo.id → base branch; '' means the repo default.
  const [targetByRepo, setTargetByRepo] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const {
    mainRepos, selectedRepos,
    isSelected, isPending, toggleRepo, deselect, loadRepos,
  } = useRepoPicker({
    onSelect: (repo) =>
      setBranchByRepo((p) => (p[repo.id] ? p : { ...p, [repo.id]: defaultBranch })),
    onDeselect: (repo) => {
      setBranchByRepo((p) => { const n = { ...p }; delete n[repo.id]; return n; });
      setTargetByRepo((p) => { const n = { ...p }; delete n[repo.id]; return n; });
    },
    onError: (msg) => setError(msg),
  });

  useEffect(() => { loadRepos(); }, [loadRepos]);

  if (!activeTask) return null;

  const addable = mainRepos.filter(
    (mr) => !activeRepos.some((r) => r.local_path === mr.local_path)
  );

  const submit = async () => {
    if (selectedRepos.length === 0) return;
    setLoading(true);
    setError('');
    const shortId = activeTask.short_id;
    try {
      const newIds = selectedRepos.map((r) => r.id);
      // set_task_repos replaces the whole set.
      const mergedIds = [...activeRepos.map((r) => r.id), ...newIds];

      if (isExplorer) {
        await invoke('set_task_repos', { shortId, repoIds: mergedIds });
        await invoke('provision_worktrees', {
          taskId: shortId,
          branches: newIds.map((id) => ({ repo_id: id, branch_name: null })),
        });
      } else {
        const specs = selectedRepos.map((r) => {
          const typed = (branchByRepo[r.id] ?? '').trim();
          const branch = typed || defaultBranch;
          return { repo: r, branch, target: (targetByRepo[r.id] ?? '').trim() };
        });

        const taken: string[] = [];
        for (const s of specs) {
          const exists = await invoke<boolean>('remote_branch_exists', {
            repoId: s.repo.id,
            branch: s.branch,
          });
          if (exists) taken.push(`${s.repo.project} → ${s.branch}`);
        }
        if (taken.length > 0) {
          setError(`Remote branch already exists on origin: ${taken.join(', ')}. Pick another name.`);
          return;
        }

        await invoke('set_task_repos', { shortId, repoIds: mergedIds });
        await invoke('provision_worktrees', {
          taskId: shortId,
          branches: specs.map((s) => ({
            repo_id: s.repo.id,
            branch_name: s.branch,
            target_branch: s.target || null,
          })),
        });
      }
      // The worktrees are on disk by now: a failed refresh must not hold the modal open.
      try {
        // Re-hydrates activeRepos / activeWorktrees via workspace_ready.
        await invoke('open_task', { shortId });
      } catch (e) {
        notify({
          kind: 'attention',
          source: 'app',
          taskId: shortId,
          title: 'Repo added, but the workspace did not refresh',
          detail: `Reopen the session to see it. ${errorText(e)}`,
        });
      }
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="wizard-overlay" onClick={onClose}>
      <div className="wizard-modal wide" onClick={(e) => e.stopPropagation()}>
        <div className="wizard-header">
          <div className="wizard-title">Add repo to {activeTask.short_id}</div>
          <div className="wizard-subtitle">{activeTask.title}</div>
          <button className="wizard-close" onClick={onClose}>×</button>
        </div>

        <div className="wizard-body">
          <p className="wizard-desc">
            {isExplorer ? (
              <>Select repositories to add — each gets a worktree on this explorer's own
              branch, renamed to the task branch if you turn this into a task.</>
            ) : (
              <>Select repositories to add, then name each branch (defaults to{' '}
              <code>{defaultBranch}</code>) and pick the base it cuts from. Creation is
              blocked if the branch already exists on the repo's origin.</>
            )}
          </p>

          {addable.length === 0 ? (
            <p className="wizard-empty">
              {mainRepos.length === 0
                ? 'No repos in the pool yet — clone one below.'
                : 'Every pooled repo is already on this task. Clone a new one below.'}
            </p>
          ) : (
            <RepoPickerSearch
              repos={addable}
              isSelected={isSelected}
              isPending={isPending}
              onToggle={toggleRepo}
            />
          )}

          {!isExplorer && selectedRepos.length > 0 && (
            <div className="wizard-branch-list">
              {selectedRepos.map((r) => (
                <RepoBranchRow
                  key={r.id}
                  repo={r}
                  defaultBranch={defaultBranch}
                  branch={branchByRepo[r.id] ?? defaultBranch}
                  onBranch={(v) => setBranchByRepo((p) => ({ ...p, [r.id]: v }))}
                  target={targetByRepo[r.id] ?? ''}
                  onTarget={(v) => setTargetByRepo((p) => ({ ...p, [r.id]: v }))}
                  onRemove={() => deselect(r.local_path)}
                />
              ))}
            </div>
          )}

          <CloneRepoForm onCloned={(repo) => { loadRepos(); toggleRepo(repo); }} />

          {error && <div className="wizard-error">{error}</div>}

          <div className="wizard-footer">
            <button className="btn-secondary" onClick={onClose}>Cancel</button>
            <button
              className="btn-primary"
              onClick={submit}
              disabled={loading || selectedRepos.length === 0}
            >
              {loading
                ? 'Adding…'
                : `Add ${selectedRepos.length || ''} repo${selectedRepos.length === 1 ? '' : 's'}`.trim()}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
