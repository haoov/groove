import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useSession, useStore } from '../shared/store';
import { BranchPicker, useOriginBranches } from './branchPicker';

/**
 * A second worktree on a repo the session already holds — the same repo, another
 * branch. Sibling of AddRepoModal: that one attaches a NEW repo, this one only
 * takes a branch, because the repo is the one you are looking at.
 *
 * The field starts from the branch provisioning itself would derive (asked for,
 * never rebuilt here), stepped `-2`, `-3`… past the branches this repo already
 * has. Within one task a numeric suffix is unambiguous — it separates worktrees,
 * not tasks.
 */
export function AddWorktreeModal({ onClose }: { onClose: () => void }) {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepoId = useSession((s) => s.activeRepoId);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const notify = useStore((s) => s.notify);

  const [typed, setTyped] = useState('');
  // '' means the repo default.
  const [target, setTarget] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const seeded = useRef(false);

  const repo = activeRepos.find((r) => r.id === activeRepoId) ?? activeRepos[0];
  const origin = useOriginBranches(repo?.id);

  // The branches this repo already has here: a repeat would silently land on the
  // existing worktree (the row upserts on session+repo+branch) and read as a no-op.
  const taken = useMemo(
    () => activeWorktrees.filter((w) => w.repo_id === repo?.id).map((w) => w.branch),
    [activeWorktrees, repo?.id],
  );

  // Seeded once from the backend's own convention — the same value the first
  // worktree got, stepped past whatever is already checked out.
  useEffect(() => {
    const shortId = activeTask?.short_id;
    if (!shortId || seeded.current) return;
    invoke<string>('default_branch_for_session', { shortId })
      .then((base) => {
        const root = base.trim();
        if (seeded.current || !root) return;
        seeded.current = true;
        let candidate = root;
        for (let n = 2; taken.includes(candidate); n++) candidate = `${root}-${n}`;
        setTyped(candidate);
      })
      .catch(() => { /* leave it empty; the user types their own */ });
  }, [activeTask?.short_id, taken]);

  const branch = typed.trim();
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
        taskId: activeTask.short_id,
        branches: [{ repo_id: repo.id, branch_name: branch, target_branch: target || null }],
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
      <div className="wizard-modal wide" onClick={(e) => e.stopPropagation()}>
        <div className="wizard-header">
          <div className="wizard-title">Add worktree to {activeTask.short_id}</div>
          <div className="wizard-subtitle">{repo.project}</div>
          <button className="wizard-close" onClick={onClose}>×</button>
        </div>

        <div className="wizard-body">
          <p className="wizard-desc">
            Another branch of <code>{repo.project}</code>, checked out beside the ones this
            session already has. Creation is blocked if the branch already exists on origin.
          </p>

          <label className="firstrun-field">
            <span className="firstrun-label">Base branch</span>
            <BranchPicker state={origin} value={target} onChange={setTarget} />
            <span className="firstrun-hint">
              Cut from this branch, and the MR targets it. Empty uses the repo default.
            </span>
          </label>

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
            {clash && (
              <span className="firstrun-hint">
                <code>{branch}</code> is already checked out here.
              </span>
            )}
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
