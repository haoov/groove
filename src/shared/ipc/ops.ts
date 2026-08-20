// Confirmation-bridge op_type names — hand-mirrored with src-tauri/src/ops.rs.

export const OP = {
  GIT_COMMIT: 'git.commit',
  GIT_PUSH: 'git.push',
  GIT_PULL: 'git.pull',
  GIT_REBASE: 'git.rebase',
  GIT_DISCARD: 'git.discard',
  GIT_DISCARD_ALL: 'git.discard_all',

  MR_CREATE: 'mr.create',
  MR_UPDATE: 'mr.update',
  MR_CLOSE: 'mr.close',

  NOTION_STATUS: 'notion.status',
  NOTION_PROPERTY: 'notion.property',
  NOTION_HOURS: 'notion.hours',
  NOTION_BODY: 'notion.body',

  TASK_CREATE: 'task.create',
  TASK_ADD_REPO: 'task.add_repo',
  TASK_CREATE_FROM_EXPLORER: 'task.create_from_explorer',
} as const;

/** Op_type prefix for git operations (commit/push/pull/rebase). */
export const OP_GIT_PREFIX = 'git.';
/** Op_type prefix for merge-request operations (create/update/close). */
export const OP_MR_PREFIX = 'mr.';
