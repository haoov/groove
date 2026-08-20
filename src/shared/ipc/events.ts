// Backend → frontend event names. Hand-mirrored with src-tauri/src/core/events.rs
// (the backend keeps that contract in one file); a name here must match one there.

import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export const EV = {
  // workspace / session lifecycle
  workspaceStub: 'workspace_stub',
  workspaceReady: 'workspace_ready',
  taskPaused: 'task_paused',
  taskFinished: 'task_finished',
  explorerDiscarded: 'explorer_discarded',
  // approvals
  confirmationRequested: 'confirmation_requested',
  confirmationResolved: 'confirmation_resolved',
  // git / worktrees
  worktreeClosed: 'worktree_closed',
  rebaseDone: 'rebase_done',
  rebaseConflict: 'rebase_conflict',
  // annotations
  annotationResolved: 'annotation_resolved',
  annotationCreated: 'annotation_created',
  // pty (agent / terminal)
  ptyStarted: 'pty_started',
  ptyOutput: 'pty_output',
  ptyExit: 'pty_exit',
  // agent activity (from Claude Code hooks)
  agentActivity: 'agent_activity',
  // backend notices
  backendNotice: 'backend_notice',
} as const;

export type EventName = (typeof EV)[keyof typeof EV];

/** Subscribe to a backend event; resolves to an unlisten function. */
export function on<T>(event: EventName, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload));
}
