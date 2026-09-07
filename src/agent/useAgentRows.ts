import { useMemo } from 'react';
import { useStore, useSessionSummaries } from '../shared/store';
import { buildAgentRows, type AgentRow } from '../shared/lib/agents';

/** The running-agents rows, rebuilt only when a session or an agent changes. */
export function useAgentRows(): AgentRow[] {
  const sessions = useSessionSummaries();
  const activeSessionId = useStore((s) => s.activeSessionId);
  const activity = useStore((s) => s.agentActivity);
  return useMemo(
    () => buildAgentRows(sessions, activeSessionId, activity),
    [sessions, activeSessionId, activity],
  );
}
