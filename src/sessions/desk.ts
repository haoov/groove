// The desk: one agent, no workspace.
//
// A place to ask a question or file a task without opening a session for it. It
// needs a session because everything the agent machinery keys off — the PTY, the
// MCP `?task=` binding, hook-reported activity, `sendToAgent` — is per session.
// The reworked backend has no desk row: all of that keys off the task-id STRING,
// so the desk is a store-only session with a fixed synthetic id.

import { useStore } from '../shared/store';
import type { Task } from '../shared/ipc/ipc';

const DESK_TASK: Task = {
  short_id: 'desk',
  notion_page_id: '',
  title: 'Desk',
  status: '',
  priority: null,
  last_synced_at: 0,
};

/** The desk's store session id, once created. */
export function deskSessionId(): string | null {
  const s = useStore.getState();
  return s.sessionOrder.find((id) => s.sessions[id]?.kind === 'desk') ?? null;
}

/**
 * The desk's session id, reactively.
 *
 * Returns a string (not a derived array), so the subscription stays referentially
 * stable — a selector that built a filtered list would re-render its component on
 * every unrelated store change. Callers filter with it in render instead.
 */
export function useDeskId(): string | null {
  return useStore((s) => s.sessionOrder.find((id) => s.sessions[id]?.kind === 'desk') ?? null);
}

/**
 * The desk session, created on first use.
 *
 * Registers WITHOUT focus: the desk has no workspace to show, and navigating
 * would drag the user off Home. The agent process is NOT started here — opening
 * the console does that, same as every other session.
 */
export async function ensureDeskSession(): Promise<string> {
  const existing = deskSessionId();
  if (existing) return existing;
  return useStore.getState().openSession({ kind: 'desk', task: DESK_TASK, focus: false });
}
