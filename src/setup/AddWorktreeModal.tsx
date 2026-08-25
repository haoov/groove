import { useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useSession, useStore } from '../shared/store';

/**
 * A second worktree on a repo the session already holds — the same repo, another
 * branch. Sibling of AddRepoModal: that one attaches a NEW repo, this one only
 * takes a branch, because the repo is the one you are looking at.
 *
 * The task id is appended so the branch stays traceable to its task, the way
 * provisioning's own default (`<type>/<slug>-<id>`) does.
 */
export function AddWorktreeModal({ onClose }: { onClose: () => void }) {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepoId = useSession((s) => s.activeRepoId);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const notify = useStore((s) => s.notify);

  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const repo = activeRepos.find((r) => r.id === activeRepoId) ?? activeRepos[0];
  const taskId = (activeTask?.short_id ?? '').toLowerCase();

  // What will actually be created. Shown live, so the suffix rule needs no prose.
  const branch = useMemo(() => {
    const name = typed.trim().toLowerCase().replace(/\s+/g, '-').replace(/-+$/, '');
    if (!name) return '';
    // Do not append twice when the name already carries the id.
    return name.endsWith(taskId) ? name : `${name}-${taskId}`;
  }, [typed, taskId]);

  // The branches this repo already has here: a repeat would silently land on the
  // existing worktree (the row upserts on session+repo+branch) and look like a no-op.
  const taken = activeWorktrees
    .filter((w) => w.repo_id === repo?.id)
    .map((w) => w.branch);

  const clash = !!branch && taken.includes(branch);

  const submit = async () => {
    if (!activeTask || !repo || !branch || clash) return;
    setBusy(true);
    setError('');
    try {
      // Refuse a name origin already has: provisioning would check out THEIR
      // branch, not create yours.
      const exists = await invoke<boolean>('remote_branch_exists', {
        repoId: repo.id,
        branch,
      });
      if (exists) {
        setError(`origin already has ${branch}. Pick another name.`);
        return;
      }
      await invoke('provision_worktrees', {
        shortId: activeTask.short_id,
        branches: [{ repo_id: repo.id, branch_name: branch }],
      });
      notify({ kind: 'success', source: 'git', title: `Added ${repo.project} on ${branch}` });
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  if (!activeTask || !repo) return null;

  return (
    <div className="wizard-overlay" onClick={onClose}>
      <div className="wizard-modal" onClick={(e) => e.stopPropagation()}>
        <div className="wizard-header">
          <div className="wizard-title">Add worktree to {activeTask.short_id}</div>
          <div className="wizard-subtitle">{repo.project}</div>
          <button className="wizard-close" onClick={onClose}>×</button>
        </div>

        <div className="wizard-body">
          <p className="wizard-desc">
            Another branch of <code>{repo.project}</code>, checked out beside the ones this
            session already has. The task id is appended so the branch stays traceable.
          </p>

          <label className="firstrun-field">
            <span className="firstrun-label">Branch</span>
            <input
              className="firstrun-input"
              autoFocus
              placeholder="fix/parser"
              value={typed}
              onChange={(e) => { setTyped(e.target.value); setError(''); }}
              onKeyDown={(e) => { if (e.key === 'Enter' && branch && !clash) submit(); }}
            />
            <span className="firstrun-hint">
              {branch
                ? <>Creates <code>{branch}</code>{clash && <> — already checked out here.</>}</>
                : <>Creation is blocked if the branch already exists on origin.</>}
            </span>
          </label>

          {taken.length > 0 && (
            <p className="wizard-desc">
              Already checked out: {taken.map((b, i) => (
                <span key={b}>{i > 0 && ', '}<code>{b}</code></span>
              ))}
            </p>
          )}

          {error && <div className="wizard-error">{error}</div>}

          <div className="wizard-footer">
            <button className="btn-secondary" onClick={onClose}>Cancel</button>
            <button className="btn-primary" onClick={submit} disabled={!branch || clash || busy}>
              {busy ? 'Adding…' : 'Add worktree'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
