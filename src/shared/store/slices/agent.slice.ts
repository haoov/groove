import { invoke } from '../../ipc/invoke';
import type { StateCreator } from 'zustand';
import type { AgentActivity } from '../../ipc/ipc';
import type { AppState, AgentSlice } from '../types';

export const agentSlice: StateCreator<AppState, [], [], AgentSlice> = (set) => ({
  agentActivity: {},
  setAgentActivity: (a) =>
    set((s) => ({ agentActivity: { ...s.agentActivity, [a.task_id]: a } })),
  dropAgentActivity: (taskId) =>
    set((s) => {
      if (!(taskId in s.agentActivity)) return {};
      const next = { ...s.agentActivity };
      delete next[taskId];
      return { agentActivity: next };
    }),
  hydrateAgentActivity: async () => {
    try {
      const rows = await invoke<AgentActivity[]>('get_agent_activity');
      set({ agentActivity: Object.fromEntries(rows.map((r) => [r.task_id, r])) });
    } catch {
      // Best-effort: an empty map just means "unknown", which is the honest state.
    }
  },
  consoleOpen: false,
  setConsoleOpen: (v) =>
    // Closing drops the maximize too: reopening straight into a full-screen agent
    // is not what the last Alt+A meant.
    set(v ? { consoleOpen: true } : { consoleOpen: false, agentMaximized: false }),
  consoleFocusNonce: 0,
  requestConsoleFocus: () =>
    set((s) => ({ consoleOpen: true, consoleFocusNonce: s.consoleFocusNonce + 1 })),
  agentMaximized: false,
  setAgentMaximized: (v) => set({ agentMaximized: v }),
});
