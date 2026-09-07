// The app store: feature slices composed into one state.
// This barrel is the only import surface; components never reach into ./slices, ./types or ./session.

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { createContext, useContext } from 'react';
import type { AppState, SessionState, SessionView } from './types';
import type { LayoutNode, SplitDir } from '../lib/layout';
import { uiSlice } from './slices/ui.slice';
import { homeSlice } from './slices/home.slice';
import { sessionsSlice, buildView } from './slices/sessions.slice';
import { agentSlice } from './slices/agent.slice';
import { confirmationsSlice } from './slices/confirmations.slice';
import { configSlice } from './slices/config.slice';
import { keybindingsSlice } from './slices/keybindings.slice';
import { notificationsSlice } from './slices/notifications.slice';
import { skillsSlice } from './slices/skills.slice';

export type { LayoutNode, SplitDir };
export { isTerminalPane } from './session';
export { sessionActions } from './slices/sessions.slice';
export type * from './types';

export const useStore = create<AppState>((...a) => ({
  ...uiSlice(...a),
  ...homeSlice(...a),
  ...sessionsSlice(...a),
  ...agentSlice(...a),
  ...confirmationsSlice(...a),
  ...configSlice(...a),
  ...keybindingsSlice(...a),
  ...notificationsSlice(...a),
  ...skillsSlice(...a),
}));

// ─── Session context + hooks ────────────────────────────────────────────────────

/** The session descendants read. Absent: hooks fall back to the active session. */
export const SessionIdContext = createContext<string | null>(null);

/** Reads from the contextual session: its fields merged with its bound actions.
 *  Always select a single value, never the whole view. */
export function useSession<T>(selector: (s: SessionView) => T): T {
  const ctx = useContext(SessionIdContext);
  return useStore((root) => selector(buildView(root, ctx ?? root.activeSessionId)));
}

// ─── Session summaries ──────────────────────────────────────────────────────────

/** What a session row needs: identity only, never its diff, panes or status. */
export interface SessionSummary {
  id: string;
  kind: SessionState['kind'];
  title: string;
  task: SessionState['task'];
  mrs: SessionState['mrs'];
}

// Kept by id while the fields hold, so `useShallow` sees an unchanged array.
const summaryCache = new Map<string, SessionSummary>();

function summaryOf(s: SessionState): SessionSummary {
  const prev = summaryCache.get(s.id);
  if (prev && prev.kind === s.kind && prev.title === s.title && prev.task === s.task && prev.mrs === s.mrs) {
    return prev;
  }
  const next: SessionSummary = { id: s.id, kind: s.kind, title: s.title, task: s.task, mrs: s.mrs };
  summaryCache.set(s.id, next);
  return next;
}

/** One row per open session, in order. */
export function useSessionSummaries(): SessionSummary[] {
  return useStore(
    useShallow((s: AppState) => {
      const rows = s.sessionOrder
        .map((id) => s.sessions[id])
        .filter((x): x is SessionState => !!x)
        .map(summaryOf);
      if (summaryCache.size > rows.length) {
        const live = new Set(rows.map((r) => r.id));
        for (const id of summaryCache.keys()) if (!live.has(id)) summaryCache.delete(id);
      }
      return rows;
    }),
  );
}

// ─── Non-hook accessors ─────────────────────────────────────────────────────────

export function getSession(state: AppState, id: string | null): SessionState | null {
  return id ? state.sessions[id] ?? null : null;
}
export function getActiveSession(state: AppState): SessionState | null {
  return getSession(state, state.activeSessionId);
}
export function findSessionByTask(state: AppState, shortId: string): SessionState | null {
  const id = state.sessionOrder.find((sid) => state.sessions[sid]?.task?.short_id === shortId);
  return id ? state.sessions[id] : null;
}
export function findSessionByPty(state: AppState, ptySessionId: string): SessionState | null {
  const id = state.sessionOrder.find((sid) =>
    state.sessions[sid]?.ptySessions.some((p) => p.sessionId === ptySessionId)
  );
  return id ? state.sessions[id] : null;
}
export function findSessionByWorktree(state: AppState, worktreeId: string): SessionState | null {
  const id = state.sessionOrder.find((sid) =>
    state.sessions[sid]?.worktrees.some((w) => w.id === worktreeId)
  );
  return id ? state.sessions[id] : null;
}
