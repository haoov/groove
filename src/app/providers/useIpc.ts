import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { EVENT } from '../../shared/ipc/events';
import { onConfirmationRequested, onConfirmationResolved } from './ipc/confirmations';
import { onRebaseConflict, onRebaseDone } from './ipc/git';
import {
  onWorkspaceStub,
  onWorkspaceReady,
  onTaskPaused,
  onTaskFinished,
  onWorktreeClosed,
  onBackendNotice,
  onAgentActivity,
  onExplorerDiscarded,
} from './ipc/sessions';
import { onPtyStarted, onPtyOutput, onPtyExit } from './ipc/pty';
import { onAnnotationResolved, onAnnotationCreated, onAnnotationUpdated } from './ipc/annotations';

/** One EVENT → handler row; the thunk registers the listener. */
function on<T>(event: string, handle: (payload: T) => void) {
  return () => listen<T>(event, ({ payload }) => handle(payload));
}

const HANDLERS = [
  on(EVENT.WORKSPACE_STUB, onWorkspaceStub),
  on(EVENT.WORKSPACE_READY, onWorkspaceReady),
  on(EVENT.TASK_PAUSED, onTaskPaused),
  on(EVENT.TASK_FINISHED, onTaskFinished),
  on(EVENT.CONFIRMATION_REQUESTED, onConfirmationRequested),
  on(EVENT.CONFIRMATION_RESOLVED, onConfirmationResolved),
  on(EVENT.WORKTREE_CLOSED, onWorktreeClosed),
  on(EVENT.PTY_STARTED, onPtyStarted),
  on(EVENT.PTY_OUTPUT, onPtyOutput),
  on(EVENT.PTY_EXIT, onPtyExit),
  on(EVENT.BACKEND_NOTICE, onBackendNotice),
  on(EVENT.AGENT_ACTIVITY, onAgentActivity),
  on(EVENT.ANNOTATION_RESOLVED, onAnnotationResolved),
  on(EVENT.ANNOTATION_CREATED, onAnnotationCreated),
  on(EVENT.ANNOTATION_UPDATED, onAnnotationUpdated),
  on(EVENT.EXPLORER_DISCARDED, onExplorerDiscarded),
  on(EVENT.REBASE_CONFLICT, onRebaseConflict),
  on(EVENT.REBASE_DONE, onRebaseDone),
];

export function useIpc() {
  useEffect(() => {
    // `listen()` can resolve after cleanup on a fast unmount; `track` unlistens those at once.
    let cancelled = false;
    const unlisten: Array<() => void> = [];
    const track = (fn: () => void) => {
      if (cancelled) fn();
      else unlisten.push(fn);
    };

    const setup = async () => {
      for (const register of HANDLERS) track(await register());
    };

    setup().catch(console.error);

    return () => {
      cancelled = true;
      unlisten.forEach((u) => u());
      unlisten.length = 0;
    };
  }, []);
}
