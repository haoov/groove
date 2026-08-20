// Tauri event names — hand-mirrored contract with src-tauri/src/events.rs.
// Every value here must match the Rust constant of the same name.

export const EVENT = {
  WORKSPACE_STUB: 'workspace_stub',
  WORKSPACE_READY: 'workspace_ready',
  TASK_PAUSED: 'task_paused',
  TASK_FINISHED: 'task_finished',
  EXPLORER_DISCARDED: 'explorer_discarded',

  CONFIRMATION_REQUESTED: 'confirmation_requested',
  CONFIRMATION_RESOLVED: 'confirmation_resolved',

  WORKTREE_CLOSED: 'worktree_closed',
  REBASE_DONE: 'rebase_done',
  REBASE_CONFLICT: 'rebase_conflict',

  ANNOTATION_RESOLVED: 'annotation_resolved',
  ANNOTATION_CREATED: 'annotation_created',

  PTY_STARTED: 'pty_started',
  PTY_OUTPUT: 'pty_output',
  PTY_EXIT: 'pty_exit',

  AGENT_ACTIVITY: 'agent_activity',
  BACKEND_NOTICE: 'backend_notice',
} as const;
