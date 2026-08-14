// "Take me to that session" — one implementation, several entry points.
//
// Notifications, the activity rail's agent list and anything else that names a
// task all mean the same thing by it, including the awkward case: the desk has no
// workspace, so focusing it as a session would open an empty one.

import { useStore, findSessionByTask } from '../store';

/**
 * Focus the session owning `taskId`, optionally opening the agent console.
 *
 * Returns false when no session holds that task (it was closed, or the event
 * arrived for something never opened) so callers can stay put rather than
 * navigating somewhere arbitrary.
 */
export function goToSession(taskId: string, opts?: { agent?: boolean }): boolean {
  const st = useStore.getState();
  const sess = findSessionByTask(st, taskId);
  if (!sess) return false;

  // The desk lives on Home behind the console — it has no workspace to show.
  if (sess.kind === 'desk') {
    st.setView('home');
    st.requestConsoleFocus();
    return true;
  }

  st.focusSession(sess.id);
  st.setView('workspace');
  if (opts?.agent) st.requestConsoleFocus();
  return true;
}
