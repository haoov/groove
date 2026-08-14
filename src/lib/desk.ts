// The desk: one agent, no workspace.
//
// A place to ask a question or file a task without opening a session for it. It
// needs a session because everything the agent machinery keys off — the PTY, the
// MCP `?task=` binding, hook-reported activity, `sendToAgent` — is per session.
// So the desk is a real (synthetic) task row and a real store session that simply
// never becomes visible as a workspace: no tab, no entry in Live.

import { invoke } from '@tauri-apps/api/core';
import { useStore } from '../store';
import type { Task } from '../types/ipc';

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

let inFlight: Promise<string> | null = null;

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
  // Concurrent callers (Home mounting twice in dev) must not create two rows.
  if (inFlight) return inFlight;

  inFlight = (async () => {
    try {
      const task = await invoke<Task>('ensure_desk_session');
      return useStore.getState().openSession({ kind: 'desk', task, focus: false });
    } finally {
      inFlight = null;
    }
  })();
  return inFlight;
}
