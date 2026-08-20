import { invoke } from '../ipc/invoke';
import { useStore } from '../store';
import { disposeHost } from './terminalHost';

/**
 * Fully close a workspace session: stop its agent/terminal PTYs (the explicit
 * "I'm done" signal — switching away never does this), drop their output
 * handlers, then remove the session from the store.
 */
export async function endSession(sessionId: string) {
  const sess = useStore.getState().sessions[sessionId];
  if (sess) {
    for (const p of sess.ptySessions) {
      try {
        await invoke('stop_agent_session', { sessionId: p.sessionId });
      } catch {
        // already dead — clean up frontend state anyway
      }
      disposeHost(p.sessionId);
    }
  }
  useStore.getState().closeSession(sessionId);
}
