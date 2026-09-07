import { useStore, findSessionByWorktree, sessionActions } from '../../../shared/store';
import type { RebaseConflictEvent, RebaseDoneEvent } from '../../../shared/ipc/ipc';

/** Records a rebase conflict on the owning session and announces it. */
export function applyRebaseConflict(sessionId: string, worktreeId: string, files: string[]) {
  sessionActions(sessionId).setRebaseConflict({ worktreeId, files });
  const st = useStore.getState();
  st.notify({
    kind: 'attention',
    source: 'git',
    taskId: st.sessions[sessionId]?.task?.short_id,
    title: `Rebase stopped on ${files.length} conflicting ${files.length === 1 ? 'file' : 'files'}`,
    detail: files.slice(0, 6).join(', ') + (files.length > 6 ? ` +${files.length - 6} more` : ''),
    goTo: { taskId: useStore.getState().sessions[sessionId]?.task?.short_id },
  });
}

/** rebase_conflict — a `rebase --continue` stopped on conflicts. */
export function onRebaseConflict(payload: RebaseConflictEvent) {
  const sess = findSessionByWorktree(useStore.getState(), payload.worktree_id);
  if (sess) applyRebaseConflict(sess.id, payload.worktree_id, payload.files ?? []);
}

/** rebase_done — the rebase finished or was aborted. */
export function onRebaseDone(payload: RebaseDoneEvent) {
  const s = useStore.getState();
  const sess = findSessionByWorktree(s, payload.worktree_id);
  if (!sess) return;
  sessionActions(sess.id).setRebaseConflict(null);
  s.notify({
    kind: 'success',
    source: 'git',
    taskId: sess.task?.short_id,
    title: payload.aborted ? 'Rebase aborted' : 'Rebase complete',
  });
  s.invalidateDiff(sess.id);
  s.refreshStatusFor(sess.id);
}
