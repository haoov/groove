import { useStore, findSessionByTask, findSessionByPty, sessionActions } from '../../../shared/store';
import { disposeHost } from '../../../shared/lib/terminalHost';
import { deliverPtyOutput } from '../../../shared/lib/ptyRegistry';
import type { PtyOutputEvent, PtyExitEvent, PtyStartedEvent } from '../../../shared/ipc/ipc';

/** pty_started — route the new PTY into the session for its task */
export function onPtyStarted(payload: PtyStartedEvent) {
  // The sign-in shell owns its PTY; no session.
  if (payload.pty_type === 'auth') return;
  const ptyType = payload.pty_type;
  const s = useStore.getState();
  const sess = findSessionByTask(s, payload.task_id);
  if (!sess) return;
  const label = payload.pty_type === 'terminal'
    ? `terminal (${payload.task_id})`
    : `claude (${payload.task_id})`;
  s.updateSession(sess.id, (ss) => ({
    ptySessions: [
      ...ss.ptySessions,
      { sessionId: payload.session_id, taskId: payload.task_id, ptyType, label },
    ],
    activePtySessionId: payload.session_id,
  }));
}

/** pty_output — dispatch to the session's xterm handler. */
export function onPtyOutput(payload: PtyOutputEvent) {
  deliverPtyOutput(payload.session_id, payload.b64);
}

/** pty_exit — dispose the terminal host and drop the store row. */
export function onPtyExit(payload: PtyExitEvent) {
  disposeHost(payload.session_id);
  const s = useStore.getState();
  const owner = findSessionByPty(s, payload.session_id);
  if (!owner) return;
  const pty = owner.ptySessions.find((p) => p.sessionId === payload.session_id);
  sessionActions(owner.id).removePtySession(payload.session_id);
  if (pty?.ptyType === 'agent' && owner.task) s.dropAgentActivity(owner.task.short_id);
}
