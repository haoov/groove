import { useMemo } from 'react';
import { useStore } from '../shared/store';
import { buildAgentRows, type AgentRow } from '../shared/lib/agents';

/** The running-agents rows, rebuilt only when a session or an agent changes. */
export function useAgentRows(): AgentRow[] {
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const activeSessionId = useStore((s) => s.activeSessionId);
  const activity = useStore((s) => s.agentActivity);
  return useMemo(
    () => buildAgentRows(sessions, sessionOrder, activeSessionId, activity),
    [sessions, sessionOrder, activeSessionId, activity],
  );
}
