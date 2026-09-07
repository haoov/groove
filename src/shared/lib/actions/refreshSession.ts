import { invoke } from '../../ipc/invoke';
import { useStore, findSessionByTask } from '../../store';
import { errorText, isNotFound } from '../pure/appError';
import type { AgentState } from '../../ipc/ipc';

/** Reports a refresh failure. A gone worktree or session is not worth a toast. */
function reportRefreshFailure(e: unknown) {
  if (isNotFound(e)) {
    console.debug('[refresh] target is gone:', errorText(e));
    return;
  }
  useStore.getState().notify({
    kind: 'error',
    source: 'git',
    title: 'Could not refresh the session',
    detail: errorText(e),
  });
}

/** Refetches the session's diff and status, and Home when on screen. `flushCaches` also drops the
 *  backend ref caches: needed when refs moved, not when only the working tree did. */
export async function refreshSession(id: string, flushCaches = true) {
  if (flushCaches) {
    await invoke('flush_git_caches').catch(reportRefreshFailure);
  }
  const s = useStore.getState();
  s.invalidateDiff(id);
  void s.refreshStatusFor(id, reportRefreshFailure);
  if (s.view === 'home') void s.refreshHome();
}

// Agent-activity pacing: refresh only on file-editing tools, throttled, plus once when the turn ends.
const WORKING_THROTTLE_MS = 1500;
const lastRefreshAt = new Map<string, number>();
const pendingTimer = new Map<string, number>();

/** Tools that mutate the working tree. */
const mutatesTree = (tool?: string | null) => !!tool && /edit|write/i.test(tool);

export function refreshOnAgentActivity(taskId: string, state: AgentState, tool?: string | null) {
  const s = useStore.getState();
  const owner = findSessionByTask(s, taskId);
  if (!owner) return;
  const id = owner.id;
  // An agent edit moves the working tree, never a ref: the ref caches stay.
  const fire = () => {
    lastRefreshAt.set(id, Date.now());
    void refreshSession(id, false);
  };
  // Turn done: refresh once and drop any pending edit-triggered refresh.
  if (state === 'idle') {
    const t = pendingTimer.get(id);
    if (t !== undefined) { clearTimeout(t); pendingTimer.delete(id); }
    fire();
    return;
  }
  // Only a file-editing tool changes the diff.
  if (state !== 'working' || !mutatesTree(tool)) return;
  if (pendingTimer.has(id)) return;
  const since = Date.now() - (lastRefreshAt.get(id) ?? 0);
  if (since >= WORKING_THROTTLE_MS) {
    fire();
    return;
  }
  pendingTimer.set(id, window.setTimeout(() => {
    pendingTimer.delete(id);
    fire();
  }, WORKING_THROTTLE_MS - since));
}
