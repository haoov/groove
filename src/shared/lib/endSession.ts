import { invoke } from '../ipc/invoke';
import { useStore } from '../store';
import { disposeHost } from './terminalHost';

/** Fully closes a session: stops its PTYs, drops their output handlers, removes the session from the store. */
export async function endSession(sessionId: string) {
  const sess = useStore.getState().sessions[sessionId];
  if (sess) {
    for (const p of sess.ptySessions) {
      try {
        await invoke('stop_agent_session', { sessionId: p.sessionId });
      } catch {
        // already dead
      }
      disposeHost(p.sessionId);
    }
  }
  useStore.getState().closeSession(sessionId);
}
