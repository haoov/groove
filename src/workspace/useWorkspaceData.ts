import { useEffect } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore, useSession } from '../shared/store';
import type { Annotation, DiffResult, Mr, MrThread } from '../shared/ipc/ipc';

/** Owns the per-task background data: git status, the diff summary, MRs with their threads, and annotations. */
export function useWorkspaceData() {
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const refreshStatus = useSession((s) => s.refreshStatus);
  const upsertMr = useSession((s) => s.upsertMr);
  const setMrThreadsForRepo = useSession((s) => s.setMrThreadsForRepo);
  const setAnnotations = useSession((s) => s.setAnnotations);
  const diffMode = useSession((s) => s.diffMode);
  const diffNonce = useSession((s) => s.diffNonce);
  const mrNonce = useSession((s) => s.mrNonce);
  const setDiff = useSession((s) => s.setDiff);
  const setLastError = useStore((s) => s.setLastError);

  // Git status for every active worktree (re-runs when the worktree set changes).
  useEffect(() => {
    if (activeTask) refreshStatus();
  }, [activeTask, activeWorktrees, refreshStatus]);

  // Diff summary for the whole task.
  useEffect(() => {
    if (!activeTask) return;
    // A superseded fetch is ignored.
    let stale = false;
    invoke<DiffResult>('get_task_diff_summary', { taskId: activeTask.short_id, mode: diffMode })
      .then((d) => { if (!stale) setDiff(d); })
      .catch((e) => { if (!stale) setLastError(e); });
    return () => { stale = true; };
  }, [activeTask, diffMode, diffNonce, setDiff, setLastError]);

  // MR and threads per repo; re-runs on `mrNonce`.
  useEffect(() => {
    if (!activeTask || !activeWorktrees.length) return;
    let stale = false;
    activeWorktrees.forEach(async (wt) => {
      try {
        const mrs = await invoke<Mr[]>('get_mr', { worktreeId: wt.id });
        if (stale) return;
        const mr = mrs[0] ?? null;
        if (mr) {
          upsertMr(mr);
          try {
            const fetched = await invoke<MrThread[]>('get_mr_threads', { mrId: mr.id });
            if (!stale) setMrThreadsForRepo(wt.repo_id, fetched);
          } catch (e) {
            console.error('[get_mr_threads]', e);
          }
        } else {
          setMrThreadsForRepo(wt.repo_id, []);
        }
      } catch {
        /* no MR for this worktree */
      }
    });
    return () => { stale = true; };
  }, [activeTask, activeWorktrees, mrNonce, upsertMr, setMrThreadsForRepo]);


  // All annotations for the task, every repo.
  useEffect(() => {
    if (!activeTask) return;
    invoke<Annotation[]>('get_annotations', { sessionId: activeTask.short_id, repoId: null })
      .then(setAnnotations)
      .catch((e) => setLastError(e));
  }, [activeTask, setAnnotations, setLastError]);
}
