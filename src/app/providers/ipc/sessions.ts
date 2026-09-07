import {
  useStore,
  findSessionByTask,
  type NotificationKind,
  type NotificationSource,
} from '../../../shared/store';
import { endSession } from '../../../shared/lib/endSession';
import { refreshOnAgentActivity } from '../../../shared/lib/refreshSession';
import type {
  AgentActivity,
  WorkspaceStubEvent,
  WorkspaceReadyEvent,
  WorktreeClosedEvent,
  TaskPausedEvent,
  TaskFinishedEvent,
} from '../../../shared/ipc/ipc';

/** workspace_stub — first open: the task has no worktrees yet. */
export function onWorkspaceStub(payload: WorkspaceStubEvent) {
  const s = useStore.getState();
  s.upsertTask(payload.task);
  s.openSession({
    kind: payload.kind ?? 'task',
    task: payload.task,
    worktrees: [],
    repos: [],
    focus: payload.focus ?? true,
  });
}

/** workspace_ready — mount or refresh a session that has worktrees; `focus` says which. */
export function onWorkspaceReady(payload: WorkspaceReadyEvent) {
  const s = useStore.getState();
  // An explorer's synthetic task stays out of the global task list.
  if ((payload.kind ?? 'task') === 'task') s.upsertTask(payload.task);
  s.openSession({
    kind: payload.kind ?? 'task',
    task: payload.task,
    worktrees: payload.worktrees,
    repos: payload.repos,
    focus: payload.focus ?? true,
  });
}

/** task_paused — close the matching session (kills its PTYs) */
export function onTaskPaused(payload: TaskPausedEvent) {
  const sess = findSessionByTask(useStore.getState(), payload.short_id);
  if (sess) endSession(sess.id);
}

/** task_finished — update local task status, then close its session */
export function onTaskFinished(payload: TaskFinishedEvent) {
  const s = useStore.getState();
  const existing = s.tasks.find((t) => t.short_id === payload.short_id);
  if (existing) s.upsertTask({ ...existing, status: payload.done_status });
  const sess = findSessionByTask(s, payload.short_id);
  if (sess) endSession(sess.id);
}

/** worktree_closed — patch the owning session's worktrees/repos */
export function onWorktreeClosed(payload: WorktreeClosedEvent) {
  const s = useStore.getState();
  const sess = findSessionByTask(s, payload.session_id);
  if (!sess) return;
  s.updateSession(sess.id, (ss) => {
    const worktrees = ss.worktrees.filter((w) => w.id !== payload.worktree_id);
    // The repo goes only with its last worktree.
    const keepRepo = worktrees.some((w) => w.repo_id === payload.repo_id);
    const repos = keepRepo ? ss.repos : ss.repos.filter((r) => r.id !== payload.repo_id);
    return {
      worktrees,
      repos,
      activeRepoId: !keepRepo && ss.activeRepoId === payload.repo_id
        ? (repos[0]?.id ?? null)
        : ss.activeRepoId,
      activeWorktreeId: ss.activeWorktreeId === payload.worktree_id
        ? (worktrees.find((w) => w.repo_id === payload.repo_id)?.id ?? worktrees[0]?.id ?? null)
        : ss.activeWorktreeId,
    };
  });
}

/** backend_notice — a problem in work the user did not trigger. */
export function onBackendNotice(payload: {
  kind: NotificationKind;
  source: NotificationSource;
  title: string;
  detail: string | null;
  task_id: string | null;
}) {
  useStore.getState().notify({
    kind: payload.kind,
    source: payload.source,
    title: payload.title,
    detail: payload.detail ?? undefined,
    taskId: payload.task_id ?? undefined,
    goTo: payload.task_id ? { taskId: payload.task_id } : undefined,
  });
}

/**
 * agent_activity — from Claude Code hooks. Toast only on the transition into
 * waiting, and only for an unfocused session.
 */
export function onAgentActivity(payload: AgentActivity) {
  const s = useStore.getState();
  const previous = s.agentActivity[payload.task_id]?.state;
  s.setAgentActivity(payload);
  // Agent activity drives the diff refresh: throttled while working, immediate once the turn ends.
  refreshOnAgentActivity(payload.task_id, payload.state, payload.tool?.name);
  if (payload.state !== 'waiting' || previous === 'waiting') return;
  const owner = findSessionByTask(s, payload.task_id);
  const focused = owner && s.activeSessionId === owner.id && s.view === 'workspace';
  if (focused) return;
  s.notify({
    kind: 'attention',
    source: 'agent',
    taskId: payload.task_id,
    title: `${payload.task_id} needs you`,
    detail: payload.tool
      ? `${payload.tool.name}${payload.tool.detail ? `(${payload.tool.detail})` : ''}`
      : 'Waiting for input',
    goTo: { taskId: payload.task_id, agent: true },
  });
}

/** explorer_discarded — end the open session, PTYs included. */
export function onExplorerDiscarded(payload: { short_id: string }) {
  const sess = findSessionByTask(useStore.getState(), payload.short_id);
  if (sess) endSession(sess.id);
}
