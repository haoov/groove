import { invoke } from '../../../shared/ipc/invoke';
import {
  useStore,
  getActiveSession,
  findSessionByTask,
  type NotificationSource,
} from '../../../shared/store';
import { OP, OP_GIT_PREFIX, OP_MR_PREFIX, OP_TASK_PREFIX } from '../../../shared/ipc/ops';
import { takeCommitPush } from '../../../shared/lib/gitChain';
import type {
  Task,
  ConfirmationRequestedEvent,
  ConfirmationResolvedEvent,
} from '../../../shared/ipc/ipc';
import { applyRebaseConflict } from './git';

/** Human op label for failure toasts, e.g. "Commit failed: …". */
function opLabel(opType: string): string {
  switch (opType) {
    case OP.GIT_COMMIT: return 'Commit';
    case OP.GIT_PUSH: return 'Push';
    case OP.GIT_PULL: return 'Pull';
    case OP.GIT_REBASE: return 'Rebase';
    case OP.GIT_DISCARD: return 'Discard';
    case OP.GIT_DISCARD_ALL: return 'Discard all';
    case OP.MR_CREATE: return 'Create merge request';
    case OP.MR_UPDATE: return 'Update merge request';
    case OP.MR_CLOSE: return 'Close merge request';
    case OP.TASK_PROPERTY: return 'Task property update';
    case OP.TASK_HOURS: return 'Hours log';
    case OP.TASK_BODY: return 'Task description update';
    case OP.TASK_CREATE: return 'Task creation';
    case OP.TASK_ADD_REPO: return 'Add repo';
    case OP.TASK_CREATE_FROM_EXPLORER: return 'Task creation';
    default: return opType;
  }
}

/** Which subsystem an op belongs to, for the notification's source chip. */
function opSource(opType: string): NotificationSource {
  if (opType.startsWith(OP_GIT_PREFIX)) return 'git';
  if (opType.startsWith(OP_MR_PREFIX)) return 'mr';
  if (opType.startsWith(OP_TASK_PREFIX)) return 'task';
  return 'app';
}

/** Human success message for an approved write op, or null if it shouldn't toast. */
function successToastFor(opType: string, result: unknown): string | null {
  switch (opType) {
    case OP.GIT_COMMIT: return 'Changes committed';
    case OP.GIT_PUSH: return 'Pushed to origin';
    case OP.GIT_PULL: return 'Pulled from origin';
    case OP.GIT_DISCARD: return 'Changes discarded';
    case OP.GIT_DISCARD_ALL: return 'All changes discarded';
    case OP.GIT_REBASE: {
      const r = result as { status?: string } | null;
      return r?.status === 'conflict' ? null : 'Rebased onto the base branch';
    }
    case OP.MR_CREATE: return 'Merge request created';
    case OP.MR_UPDATE: return 'Merge request updated';
    case OP.MR_CLOSE: return 'Merge request closed';
    case OP.TASK_PROPERTY: return 'Task property updated';
    case OP.TASK_HOURS: return 'Hours logged';
    case OP.TASK_BODY: return 'Task description updated';
    case OP.TASK_ADD_REPO: return 'Repo added to the task';
    case OP.TASK_CREATE: {
      const t = result as { short_id?: string } | null;
      return t?.short_id ? `Task ${t.short_id} created` : 'Task created';
    }
    case OP.TASK_CREATE_FROM_EXPLORER: {
      const t = result as { short_id?: string } | null;
      return t?.short_id ? `Task ${t.short_id} created` : 'Task created';
    }
    default: return null;
  }
}

/** confirmation_requested — attribute to the task the backend named, never the active session. */
export function onConfirmationRequested(payload: ConfirmationRequestedEvent) {
  const s = useStore.getState();

  // autoApprove on the owning session: approve without queueing, then notify.
  const owner = payload.session_id ? findSessionByTask(s, payload.session_id) : null;
  if (owner?.autoApprove) {
    invoke('resolve_confirmation', { id: payload.id, approved: true })
      .then(() => {
        useStore.getState().notify({
          kind: 'info',
          source: payload.origin === 'mcp' ? 'mcp' : 'app',
          taskId: payload.session_id ?? undefined,
          title: `${opLabel(payload.op_type)} auto-approved`,
          detail: 'This session is set to allow every request.',
          goTo: { taskId: payload.session_id ?? undefined },
        });
      })
      .catch((e) => useStore.getState().setLastError(e));
    return;
  }

  s.addConfirmation({
    id: payload.id,
    session_id: payload.session_id,
    op_type: payload.op_type,
    payload: payload.payload,
    origin: payload.origin,
  });
}

export function onConfirmationResolved(payload: ConfirmationResolvedEvent) {
  const s = useStore.getState();
  const conf = s.pendingConfirmations.find((c) => c.id === payload.id);
  // Owner: the payload's session_id, then the pending row's, then the active session.
  const ownerTaskId = payload.session_id ?? conf?.session_id ?? null;
  const owner = ownerTaskId ? findSessionByTask(s, ownerTaskId) : getActiveSession(s);
  s.removeConfirmation(payload.id);
  // Commit & Push: the chained push, if one was queued.
  const chainedPushWt = payload.op_type === OP.GIT_COMMIT ? takeCommitPush(payload.id) : undefined;
  if (!payload.approved) return;
  if (chainedPushWt && !payload.error) {
    invoke('push', { worktreeId: chainedPushWt }).catch((e) => s.setLastError(e));
  }

  // Approved but failed: surface the error, then refresh.
  if (payload.error) {
    s.notify({
      kind: 'error',
      source: opSource(payload.op_type),
      taskId: ownerTaskId ?? undefined,
      title: `${opLabel(payload.op_type)} failed`,
      detail: payload.error,
      goTo: { taskId: ownerTaskId ?? undefined },
    });
    if (owner) {
      s.invalidateDiff(owner.id);
      s.refreshStatusFor(owner.id);
      if (payload.op_type.startsWith(OP_MR_PREFIX)) s.invalidateMrs(owner.id);
    }
    return;
  }

  const done = successToastFor(payload.op_type, payload.result);
  if (done) {
    s.notify({
      kind: 'success',
      source: opSource(payload.op_type),
      taskId: ownerTaskId ?? undefined,
      title: done,
      goTo: { taskId: ownerTaskId ?? undefined },
    });
  }

  if (s.view === 'home') s.refreshHome();

  // Explorer → task conversion: flip the owning session to a task session, PTYs kept.
  if (payload.op_type === OP.TASK_CREATE_FROM_EXPLORER) {
    const result = payload.result as (Task & { branch_warnings?: string[] }) | null;
    if (result && owner) {
      s.upsertTask(result);
      s.updateSession(owner.id, (ss) => ({
        task: result,
        kind: 'task',
        title: result.short_id,
        ptySessions: ss.ptySessions.map((p) => ({ ...p, taskId: result.short_id })),
      }));
      // Conversion relocated the worktrees; re-open for the fresh paths and watchers.
      invoke('open_task', { shortId: result.short_id }).catch(console.error);
    }
    const warnings = result?.branch_warnings ?? [];
    if (warnings.length) {
      s.notify({
        kind: 'error',
        source: 'git',
        taskId: result?.short_id,
        title: 'Branch not switched in some worktrees',
        detail: warnings.join('\n'),
      });
    }
    return;
  }

  if (typeof payload.op_type === 'string' && payload.op_type.startsWith(OP_GIT_PREFIX) && owner) {
    s.invalidateDiff(owner.id);
    s.refreshStatusFor(owner.id);
    // A push also changes the MR's CI state.
    if (payload.op_type === OP.GIT_PUSH) s.invalidateMrs(owner.id);
  }
  if (typeof payload.op_type === 'string' && payload.op_type.startsWith(OP_MR_PREFIX) && owner) {
    s.invalidateMrs(owner.id);
  }
  // Initial rebase that stopped on conflicts.
  if (payload.op_type === OP.GIT_REBASE) {
    const res = payload.result as { status?: string; files?: string[]; worktree_id?: string } | null;
    if (res?.status === 'conflict' && owner) {
      applyRebaseConflict(owner.id, res.worktree_id ?? '', Array.isArray(res.files) ? res.files : []);
    }
  }
}
