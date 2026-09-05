// "Take me to that session" — one implementation, several entry points.
//
// Notifications, the running-agents list and anything else that names a task
// all mean the same thing by it.

import { useStore, findSessionByTask } from '../store';

/**
 * Focus the session owning `taskId`, optionally opening the agent console.
 *
 * Returns false when no session holds that task (it was closed, or the event
 * arrived for something never opened) so callers can stay put rather than
 * navigating somewhere arbitrary.
 */
export function goToSession(taskId: string, opts?: { agent?: boolean }): boolean {
  const sess = findSessionByTask(useStore.getState(), taskId);
  return !!sess && goToSessionById(sess.id, opts);
}

/** The same, by session id — for a session that has no task. */
export function goToSessionById(sessionId: string, opts?: { agent?: boolean }): boolean {
  const st = useStore.getState();
  if (!st.sessions[sessionId]) return false;

  st.focusSession(sessionId);
  st.setView('workspace');
  if (opts?.agent) st.requestConsoleFocus();
  return true;
}
