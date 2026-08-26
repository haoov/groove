import { useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { GitPullRequest, Sparkles, Check, X, Trash2 } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { sendSkill } from '../shared/lib/agentSend';
import { endSession } from '../shared/lib/endSession';
import { MrOverview } from './MrOverview';

/** Review session overview: the MR overview plus the review action row
 *  (AI co-review, finish review). Approve lives on the MR overview itself. */
export function ReviewOverview() {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const mrs = useSession((s) => s.mrs);
  const sessionId = useSession((s) => s.id);
  const setLastError = useStore((s) => s.setLastError);

  const [confirmingFinish, setConfirmingFinish] = useState(false);

  const mr = mrs[0] ?? null;
  const repoId = useMemo(
    () => activeWorktrees.find((w) => w.id === mr?.worktree_id)?.repo_id ?? activeRepos[0]?.id ?? '',
    [activeWorktrees, activeRepos, mr],
  );

  // The same skill the agent pill sends. sendSkill starts the agent if there
  // isn't one — waiting on its SessionStart hook rather than guessing how long
  // Claude takes to boot.
  const coReview = async () => {
    if (!activeTask) return;
    useStore.getState().requestConsoleFocus(); // surface the conversation
    try {
      await sendSkill(sessionId, 'groove:co-review');
    } catch (e) {
      setLastError(String(e));
    }
  };

  const finishReview = async () => {
    if (!activeTask) return;
    setConfirmingFinish(false);
    try {
      // Same teardown as an explorer discard: worktree + all session rows.
      await endSession(sessionId);
      await invoke('discard_explorer', { shortId: activeTask.short_id });
    } catch (e) {
      setLastError(String(e));
    }
  };

  if (!activeTask) return null;

  return (
    <div className="review-overview">
      <div className="review-actions">
        <span className="review-actions-eyebrow">
          <GitPullRequest size={13} strokeWidth={1.75} />
          Review · {activeTask.short_id}
        </span>
        <div className="review-actions-buttons">
          <button className="finish-task-btn" onClick={coReview} title="Ask the agent to co-review: it annotates problem lines, you decide what to post">
            <Sparkles size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
            AI co-review
          </button>
          {confirmingFinish ? (
            <>
              <span className="explorer-confirm-label">Close review &amp; delete worktree?</span>
              <button className="btn-icon explorer-discard" title="Confirm" onClick={finishReview}>
                <Check size={13} strokeWidth={2} />
              </button>
              <button className="btn-icon" title="Cancel" onClick={() => setConfirmingFinish(false)}>
                <X size={13} strokeWidth={2} />
              </button>
            </>
          ) : (
            <button className="finish-task-btn review-finish-btn" onClick={() => setConfirmingFinish(true)} title="Done reviewing — clean up the local worktree and session">
              <Trash2 size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
              Finish review
            </button>
          )}
        </div>
      </div>
      {mr ? (
        <MrOverview repoId={repoId} mrId={mr.id} />
      ) : (
        <div className="overview-view">
          <div className="overview-inner">
            <p className="overview-empty-body">Loading the merge request…</p>
          </div>
        </div>
      )}
    </div>
  );
}
