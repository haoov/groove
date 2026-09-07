import { useEffect, useRef, useState } from 'react';
import {
  GitCommit, Upload, Download, ChevronsUp, ChevronDown, AlertTriangle, GitPullRequest, ExternalLink,
} from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useSession, useStore } from '../shared/store';
import type { WorktreeStatus, Mr } from '../shared/ipc/ipc';
import { registerCommitPush } from '../shared/lib/gitChain';
import { forgeName, mrSigil } from '../shared/lib/forge';
import { openExternal } from '../shared/lib/openExternal';
import { errorText } from '../shared/lib/appError';
import { MrCiChip } from './MrCiChip';

type ActionKey = 'commit' | 'commit-push' | 'push' | 'pull' | 'rebase' | 'create-mr';

const GIT_MENU: { key: ActionKey; label: string; icon: typeof GitCommit; needsMessage?: boolean }[] = [
  { key: 'commit',      label: 'Commit',         icon: GitCommit, needsMessage: true },
  { key: 'commit-push', label: 'Commit & Push',  icon: Upload,    needsMessage: true },
  { key: 'push',        label: 'Push',           icon: Upload },
  { key: 'pull',        label: 'Pull',           icon: Download },
  { key: 'rebase',      label: 'Rebase on main', icon: ChevronsUp },
  { key: 'create-mr',   label: 'Create MR…',     icon: GitPullRequest },
];

export function GitCommitPanel({
  status, branch, worktreeId, commitOnly = false, mr, onCommit, onAction,
}: {
  status?: WorktreeStatus;
  branch?: string;
  /** The active worktree; scopes the rebase-conflict banner. */
  worktreeId?: string;
  /** Explorer sessions: only commit is available. */
  commitOnly?: boolean;
  /** This branch's merge request, when it has one. Hides "Create MR…". */
  mr?: Mr;
  /** Resolves to the commit confirmation's id (chains Commit & Push). */
  onCommit: (message: string) => Promise<string | undefined | void>;
  onAction: (cmd: string) => void;
}) {
  const [message, setMessage] = useState('');
  const [menuOpen, setMenuOpen] = useState(false);
  const [committing, setCommitting] = useState(false);
  const taRef = useRef<HTMLTextAreaElement>(null);
  const hasMr = !!mr;
  const menu = commitOnly
    ? GIT_MENU.filter((a) => a.key === 'commit')
    : GIT_MENU.filter((a) => a.key !== 'create-mr' || !hasMr);

  // ── Rebase-conflict flow + commit focus ─────────────────────────────────────
  const rebaseConflict = useSession((s) => s.rebaseConflict);
  const commitFocusNonce = useStore((s) => s.commitFocusNonce);
  const [rebaseError, setRebaseError] = useState<string | null>(null);
  const initialFocusNonce = useRef(commitFocusNonce);

  // Focus the textarea on a commit request; skip the nonce present at mount.
  useEffect(() => {
    if (commitFocusNonce === initialFocusNonce.current) return;
    taRef.current?.focus();
  }, [commitFocusNonce]);

  const conflict = rebaseConflict && (!worktreeId || rebaseConflict.worktreeId === worktreeId)
    ? rebaseConflict : null;

  const runRebaseAction = async (cmd: 'rebase_continue' | 'rebase_abort') => {
    if (!conflict) return;
    setRebaseError(null);
    try {
      await invoke(cmd, { worktreeId: conflict.worktreeId });
    } catch (e) {
      setRebaseError(errorText(e));
    }
  };

  // Auto-grow the textarea, 150px maximum.
  const autosize = () => {
    const ta = taRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = `${Math.min(ta.scrollHeight, 150)}px`;
  };
  useEffect(autosize, [message]);

  const hasMsg = message.trim().length > 0;
  const ahead = status?.ahead ?? 0;
  const behind = status?.behind ?? 0;
  const dirty = (status?.modified ?? 0) + (status?.staged ?? 0);

  // The primary button's action.
  const primary: ActionKey | null = commitOnly
    ? (hasMsg ? 'commit' : null)
    : hasMsg ? 'commit' : behind > 0 ? 'pull' : ahead > 0 ? 'push' : null;

  const run = async (key: ActionKey) => {
    setMenuOpen(false);
    if (key === 'commit' || key === 'commit-push') {
      if (!hasMsg || committing) return;
      setCommitting(true);
      try {
        const confirmationId = await onCommit(message.trim());
        setMessage('');
        // The push posts only after the commit confirmation resolves approved (useIpc).
        if (key === 'commit-push' && typeof confirmationId === 'string' && worktreeId) {
          registerCommitPush(confirmationId, worktreeId);
        }
      } catch { /* the message stays */ }
      finally { setCommitting(false); }
    } else if (key === 'push') onAction('push');
    else if (key === 'pull') onAction('pull');
    else if (key === 'rebase') onAction('rebase_on_main');
    else if (key === 'create-mr') onAction('create_mr');
  };

  const primaryEntry = primary ? GIT_MENU.find((a) => a.key === primary)! : null;
  const PrimaryIcon = primaryEntry?.icon ?? GitCommit;

  return (
    <div className="git-commit-panel">
      {conflict && (
        <div className="git-warn-banner sidebar-footer-banner">
          <div className="git-warn-header">
            <AlertTriangle size={12} strokeWidth={2} style={{ color: 'var(--wb-warn)', flexShrink: 0 }} />
            <span>Rebase conflict — {conflict.files.length} file{conflict.files.length === 1 ? '' : 's'}</span>
          </div>
          <div className="git-warn-body">
            {conflict.files.map((f) => <div key={f}>{f}</div>)}
          </div>
          {rebaseError && <div className="mr-thread-resolve-error">{rebaseError}</div>}
          <button className="git-warn-close-btn" onClick={() => runRebaseAction('rebase_continue')}>
            Continue rebase
          </button>
          <button className="git-warn-close-btn" onClick={() => runRebaseAction('rebase_abort')}>
            Abort rebase
          </button>
        </div>
      )}
      {mr && (
        <div className="git-commit-mr">
          <button
            className="git-commit-mr-num"
            onClick={() => openExternal(mr.url)}
            title={`${mr.url} — open in ${forgeName(mr.platform)}`}
          >
            {mrSigil(mr.platform)}{mr.remote_id}
            <ExternalLink size={11} strokeWidth={1.75} />
          </button>
          <span className={`git-commit-mr-state mr-state-${mr.state}`}>{mr.state}</span>
          <MrCiChip mr={mr} />
        </div>
      )}
      <textarea
        ref={taRef}
        className="git-commit-textarea"
        placeholder={branch ? `Message — commit to ${branch} (Ctrl+Enter)` : 'Message (Ctrl+Enter to commit)'}
        rows={3}
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={(e) => { if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') run('commit'); }}
      />
      <div className="git-commit-actions">
        <span className="git-commit-stats">
          {dirty > 0 && status!.modified > 0 && <span className="repo-stat-modified" title={`${status!.modified} modified`}>~{status!.modified}</span>}
          {dirty > 0 && status!.staged > 0 && <span className="repo-stat-staged" title={`${status!.staged} staged`}>+{status!.staged}</span>}
          {!commitOnly && ahead > 0 && <span className="repo-stat-ahead" title={`${ahead} ahead`}>↑{ahead}</span>}
          {!commitOnly && behind > 0 && <span className="repo-stat-behind" title={`${behind} behind`}>↓{behind}</span>}
          {dirty === 0 && (commitOnly || (ahead === 0 && behind === 0)) && status && <span className="git-commit-clean">{commitOnly ? 'No changes' : 'Up to date'}</span>}
        </span>

        <div className="git-split">
          <button
            className="git-split-main"
            disabled={!primary || committing}
            onClick={() => primary && run(primary)}
            title={primaryEntry?.label ?? 'Nothing to do'}
          >
            <PrimaryIcon size={12} strokeWidth={1.75} />
            <span>{committing ? 'Working…' : primaryEntry?.label ?? 'Commit'}</span>
          </button>
          {menu.length > 1 && (
            <button className="git-split-toggle" onClick={() => setMenuOpen((o) => !o)} aria-label="Choose git action">
              <ChevronDown size={12} strokeWidth={2} />
            </button>
          )}
          {menuOpen && (
            <>
              <div className="git-split-backdrop" onClick={() => setMenuOpen(false)} />
              <div className="git-split-menu">
                {menu.map((a) => {
                  const AIcon = a.icon;
                  const disabled = a.needsMessage && !hasMsg;
                  return (
                    <button
                      key={a.key}
                      className={`git-split-item ${a.key === primary ? 'active' : ''}`}
                      disabled={disabled}
                      onClick={() => run(a.key)}
                    >
                      <AIcon size={12} strokeWidth={1.75} />
                      <span>{a.label}</span>
                    </button>
                  );
                })}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
