// Focus the session that owns a task: the one implementation behind every entry point.

import { useStore, findSessionByTask } from '../../store';

/** Focuses the session owning `taskId`, optionally opening the agent console.
 *  Returns false when no session holds that task. */
export function goToSession(taskId: string, opts?: { agent?: boolean }): boolean {
  const sess = findSessionByTask(useStore.getState(), taskId);
  return !!sess && goToSessionById(sess.id, opts);
}

/** The same, by session id. */
export function goToSessionById(sessionId: string, opts?: { agent?: boolean }): boolean {
  const st = useStore.getState();
  if (!st.sessions[sessionId]) return false;

  st.focusSession(sessionId);
  st.setView('workspace');
  if (opts?.agent) st.requestConsoleFocus();
  return true;
}
