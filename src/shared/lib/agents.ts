// The running agents: one row per open session, with what its agent is doing.
// Pure, so the store hook and the window bridge build the same rows.

import type { AgentActivity, SessionKind } from '../ipc/ipc';
import type { SessionState } from '../store/types';
import { mrRef } from './forge';

export interface AgentRow {
  sessionId: string;
  /** Task short id; null for a session without a task. */
  taskId: string | null;
  /** The id shown: the MR number for a review, else the task short id. */
  idLabel: string | null;
  title: string;
  kind: SessionKind;
  status: string | null;
  activity: AgentActivity | null;
  active: boolean;
}

/** Logical pixels. The agent column grows by the list's width while it is open. */
export const AGENTS_SIDEBAR_MIN = 240;
export const AGENTS_SIDEBAR_MAX = 360;
export const AGENTS_SIDEBAR_DEFAULT = 280;

/** A usable width, whatever was dragged or stored. */
export function clampAgentsWidth(n: unknown): number {
  if (typeof n !== 'number' || !Number.isFinite(n)) return AGENTS_SIDEBAR_DEFAULT;
  return Math.round(Math.max(AGENTS_SIDEBAR_MIN, Math.min(AGENTS_SIDEBAR_MAX, n)));
}

type SessionLite = Pick<SessionState, 'id' | 'kind' | 'title' | 'task' | 'mrs'>;

/** The short id for a session: the MR number for reviews (their short_id is long
 *  and unhelpful), else the task short_id. */
export function sessionIdLabel(s: SessionLite): string | null {
  if (s.kind === 'review') {
    const mr = s.mrs?.[0];
    if (mr) return mrRef(mr.platform, mr.remote_id);
  }
  return s.task?.short_id ?? null;
}

/** What the agent is doing, in one line. */
export function agentLine(a: AgentActivity): string {
  const tool = a.tool ? (a.tool.detail ? `${a.tool.name}(${a.tool.detail})` : a.tool.name) : null;
  switch (a.state) {
    case 'waiting':
      return tool ? `waiting · ${tool}` : 'waiting on you';
    case 'working':
      return tool ?? 'working…';
    case 'idle':
      return a.last_message ? `idle · ${a.last_message}` : 'idle';
  }
}

/** Waiting agents first, then session order. */
export function buildAgentRows(
  sessions: Record<string, SessionLite>,
  sessionOrder: string[],
  activeSessionId: string | null,
  activity: Record<string, AgentActivity>,
): AgentRow[] {
  const rows = sessionOrder
    .map((id) => sessions[id])
    .filter((s): s is SessionLite => !!s)
    .map((s): AgentRow => {
      const taskId = s.task?.short_id ?? null;
      return {
        sessionId: s.id,
        taskId,
        idLabel: sessionIdLabel(s),
        title: s.task?.title || s.title,
        kind: s.kind,
        status: s.task?.status ?? null,
        activity: taskId ? activity[taskId] ?? null : null,
        active: s.id === activeSessionId,
      };
    });
  const rank = (r: AgentRow) => (r.activity?.state === 'waiting' ? 0 : 1);
  return rows.sort((a, b) => rank(a) - rank(b));
}

export function waitingCount(rows: AgentRow[]): number {
  return rows.filter((r) => r.activity?.state === 'waiting').length;
}
