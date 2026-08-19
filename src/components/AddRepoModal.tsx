import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSession, useStore } from '../store';
import { useRepoPicker, RepoPickerList, CloneRepoForm } from './repoPicker';

/**
 * Add one or more repos to the ALREADY-OPEN task — the post-wizard path.
 * Reuses the wizard's step-1 repo picker. Branches default to the task branch
 * (same as the task's other worktrees); provisioning is incremental so existing
 * worktrees are left untouched.
 */
export function AddRepoModal({ onClose }: { onClose: () => void }) {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const isExplorer = useSession((s) => s.kind === 'explorer');
  const notify = useStore((s) => s.notify);
  const defaultBranch = (activeTask?.short_id ?? '').toLowerCase();

  // repo.id → branch name. Seeded with the task branch when a repo is selected so
  // the field holds real text you can prepend or append to; a placeholder gave
  // nothing to edit.
  const [branchByRepo, setBranchByRepo] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const {
    mainRepos, selectedRepos,
    isSelected, isPending, toggleRepo, loadRepos,
  } = useRepoPicker({
    onSelect: (repo) =>
      setBranchByRepo((p) => (p[repo.id] ? p : { ...p, [repo.id]: defaultBranch })),
    onDeselect: (repo) => setBranchByRepo((p) => { const n = { ...p }; delete n[repo.id]; return n; }),
    onError: (msg) => setError(msg),
  });

  useEffect(() => { loadRepos(); }, [loadRepos]);

  if (!activeTask) return null;

  // Only repos not already attached to this task are addable.
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
      // set_task_repos replaces the set, so merge with the repos already attached.
      const mergedIds = [...activeRepos.map((r) => r.id), ...newIds];

      if (isExplorer) {
        await invoke('set_task_repos', { shortId, repoIds: mergedIds });
        await invoke('provision_worktrees', {
          taskId: shortId,
          branches: newIds.map((id) => ({ repo_id: id, branch_name: null })),
        });
      } else {
        // Resolve each repo's target branch (typed override, or the task default).
        const specs = selectedRepos.map((r) => {
          const typed = (branchByRepo[r.id] ?? '').trim();
          return { repo: r, branch: typed || defaultBranch, custom: typed || null };
        });

        // Refuse if any target branch already exists on the repo's origin.
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
          branches: specs.map((s) => ({ repo_id: s.repo.id, branch_name: s.custom })),
        });
      }
      // The repos are attached and provisioned by this point, so a failed refresh
      // must NOT hold the modal open: it would read as "nothing happened" while
      // the worktrees are already on disk. Report it and close either way.
      try {
        // Re-hydrates activeRepos / activeWorktrees via workspace_ready.
        await invoke('open_task', { shortId });
      } catch (e) {
        notify({
          kind: 'attention',
          source: 'app',
          taskId: shortId,
          title: 'Repo added, but the workspace did not refresh',
          detail: `Reopen the session to see it. ${String(e)}`,
        });
      }
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="wizard-overlay" onClick={onClose}>
      <div className="wizard-modal" onClick={(e) => e.stopPropagation()}>
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
              <code>{activeTask.short_id.toLowerCase()}</code>). Creation is blocked if the
              branch already exists on the repo's origin.</>
            )}
          </p>

          {addable.length === 0 ? (
            <p className="wizard-empty">
              {mainRepos.length === 0
                ? 'No repos in the pool yet — clone one below.'
                : 'Every pooled repo is already on this task. Clone a new one below.'}
            </p>
          ) : (
            <RepoPickerList
              repos={addable}
              isSelected={isSelected}
              isPending={isPending}
              onToggle={toggleRepo}
              onConfirm={submit}
            />
          )}

          {!isExplorer && selectedRepos.length > 0 && (
            <div className="wizard-branch-list">
              {selectedRepos.map((r) => (
                <div key={r.id} className="wizard-branch-item">
                  <span className="wizard-branch-repo">{r.project}</span>
                  <input
                    className="wizard-input"
                    placeholder={defaultBranch}
                    value={branchByRepo[r.id] ?? defaultBranch}
                    onChange={(e) => setBranchByRepo((p) => ({ ...p, [r.id]: e.target.value }))}
                  />
                </div>
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
