import { invoke } from '@tauri-apps/api/core';
import { useStore, findSessionByTask } from '../store';
import type { AgentState } from '../ipc/ipc';

/**
 * The refresh contract (REWORK.md): flush the backend git caches, then refetch
 * the session's diff + status — and Home, when it's on screen. The ONE named
 * path shared by the agent-activity handler and the sidebar refresh button, so
 * neither can skip the cache flush and read 5s-stale refs.
 */
export async function refreshSession(id: string) {
  await invoke('flush_git_caches').catch(() => { /* best-effort */ });
  const s = useStore.getState();
  s.invalidateDiff(id);
  void s.refreshStatusFor(id);
  if (s.view !== 'workspace') void s.refreshHome();
}

// Agent-activity pacing: hooks fire on every tool call, so refreshing each one
// would hammer git. Throttled while `working`; the transition to `idle` (turn
// done — edits have landed) refreshes immediately so the final state never waits.
const WORKING_THROTTLE_MS = 3000;
const lastRefreshAt = new Map<string, number>();
const pendingTimer = new Map<string, number>();

export function refreshOnAgentActivity(taskId: string, state: AgentState) {
  const s = useStore.getState();
  const owner = findSessionByTask(s, taskId);
  if (!owner) return;
  const id = owner.id;
  const fire = () => {
    lastRefreshAt.set(id, Date.now());
    void refreshSession(id);
  };
  if (state === 'idle') {
    const t = pendingTimer.get(id);
    if (t !== undefined) { clearTimeout(t); pendingTimer.delete(id); }
    fire();
    return;
  }
  // `waiting` changes nothing on disk — only `working` ticks the throttle.
  if (state !== 'working') return;
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
