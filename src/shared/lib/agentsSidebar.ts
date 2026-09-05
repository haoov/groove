import { useStore } from '../store';

/**
 * Show or fold the running-agents list. It lives in the agent panel, so showing it
 * opens the console when that is closed; detached, it opens in the window.
 * Returns false when there is no session to list.
 */
export function toggleAgentsSidebar(): boolean {
  const st = useStore.getState();
  if (st.sessionOrder.length === 0) return false;

  const showing = st.agentDetached
    ? st.agentsSidebarOpen
    : st.view === 'workspace' && st.consoleOpen && st.agentsSidebarOpen;
  if (showing) {
    st.setAgentsSidebarOpen(false);
    return true;
  }

  st.setAgentsSidebarOpen(true);
  if (st.agentDetached) {
    // Brings the window forward (AgentWindowBridge follows the nonce).
    st.requestConsoleFocus();
    return true;
  }
  if (!st.activeSessionId) st.focusSession(st.sessionOrder[0]);
  st.setConsoleOpen(true);
  st.setView('workspace');
  return true;
}
