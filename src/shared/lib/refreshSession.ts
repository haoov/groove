import { invoke } from '../ipc/invoke';
import { useStore, findSessionByTask } from '../store';
import type { AgentState } from '../ipc/ipc';

/**
 * The refresh contract: flush the backend git caches, then refetch
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

// Agent-activity pacing: a hook fires on EVERY tool call — reads, greps and
// thinking included — so refreshing on each would hammer git for nothing. The
// disk only changes when the agent runs a file-editing tool, so refresh only on
// those (throttled to coalesce edit bursts), plus once when the turn ends.
const WORKING_THROTTLE_MS = 1500;
const lastRefreshAt = new Map<string, number>();
const pendingTimer = new Map<string, number>();

/** Tools that actually mutate the working tree (Edit / MultiEdit / NotebookEdit
 *  / Write). Everything else — Read, Grep, Bash, thinking — leaves it untouched. */
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
  // Turn done — edits have landed; refresh once, and drop any pending
  // edit-triggered refresh since this supersedes it.
  if (state === 'idle') {
    const t = pendingTimer.get(id);
    if (t !== undefined) { clearTimeout(t); pendingTimer.delete(id); }
    fire();
    return;
  }
  // Only a file-editing tool changes the diff; reads/greps/waiting never do.
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
