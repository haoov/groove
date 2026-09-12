// This file was generated from src-tauri/src/core/events.rs. Do not edit it manually.

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
  ANNOTATION_UPDATED: 'annotation_updated',
  PTY_STARTED: 'pty_started',
  PTY_OUTPUT: 'pty_output',
  PTY_EXIT: 'pty_exit',
  AGENT_ACTIVITY: 'agent_activity',
  BACKEND_NOTICE: 'backend_notice',
} as const;

export type EventName = (typeof EVENT)[keyof typeof EVENT];
