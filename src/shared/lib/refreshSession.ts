import { invoke } from '../ipc/invoke';
import { useStore, findSessionByTask } from '../store';
import type { AgentState } from '../ipc/ipc';

/** Flushes the backend git caches, then refetches the session's diff and status, and Home when on screen.
 *  The one refresh path: never skip the cache flush. */
export async function refreshSession(id: string) {
  await invoke('flush_git_caches').catch(() => { /* best-effort */ });
  const s = useStore.getState();
  s.invalidateDiff(id);
  void s.refreshStatusFor(id);
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
  const fire = () => {
    lastRefreshAt.set(id, Date.now());
    void refreshSession(id);
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
