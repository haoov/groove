import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '../../shared/ipc/invoke';
import {
  useStore,
  getActiveSession,
  findSessionByTask,
  findSessionByPty,
  findSessionByWorktree,
  sessionActions,
  type NotificationKind,
  type NotificationSource,
} from '../../shared/store';
import { EVENT } from '../../shared/ipc/events';
import { OP, OP_GIT_PREFIX, OP_MR_PREFIX, OP_TASK_PREFIX } from '../../shared/ipc/ops';
import { disposeHost } from '../../shared/lib/terminalHost';
import { deliverPtyOutput } from '../../shared/lib/ptyRegistry';
import { endSession } from '../../shared/lib/endSession';
import { refreshOnAgentActivity } from '../../shared/lib/refreshSession';
import { takeCommitPush } from '../../shared/lib/gitChain';
import type {
  AgentActivity,
  Annotation,
  Task,
  WorkspaceStubEvent,
  WorkspaceReadyEvent,
  ConfirmationRequestedEvent,
  ConfirmationResolvedEvent,
  WorktreeClosedEvent,
  RebaseConflictEvent,
  RebaseDoneEvent,
  PtyOutputEvent,
  PtyExitEvent,
  PtyStartedEvent,
  TaskPausedEvent,
  TaskFinishedEvent,
} from '../../shared/ipc/ipc';

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

/** Records a rebase conflict on the owning session and announces it. */
function applyRebaseConflict(sessionId: string, worktreeId: string, files: string[]) {
  sessionActions(sessionId).setRebaseConflict({ worktreeId, files });
  const st = useStore.getState();
  st.notify({
    kind: 'attention',
    source: 'git',
    taskId: st.sessions[sessionId]?.task?.short_id,
    title: `Rebase stopped on ${files.length} conflicting ${files.length === 1 ? 'file' : 'files'}`,
    detail: files.slice(0, 6).join(', ') + (files.length > 6 ? ` +${files.length - 6} more` : ''),
    goTo: { taskId: useStore.getState().sessions[sessionId]?.task?.short_id },
  });
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
      // workspace_stub — first open: the task has no worktrees yet.
      track(
        await listen<WorkspaceStubEvent>(EVENT.WORKSPACE_STUB, ({ payload }) => {
          const s = useStore.getState();
          s.upsertTask(payload.task);
          s.openSession({
            kind: payload.kind ?? 'task',
            task: payload.task,
            worktrees: [],
            repos: [],
            focus: payload.focus ?? true,
          });
        })
      );

      // workspace_ready — mount or refresh a session that has worktrees; `focus` says which.
      track(
        await listen<WorkspaceReadyEvent>(EVENT.WORKSPACE_READY, ({ payload }) => {
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
        })
      );

      // task_paused — close the matching session (kills its PTYs)
      track(
        await listen<TaskPausedEvent>(EVENT.TASK_PAUSED, ({ payload }) => {
          const sess = findSessionByTask(useStore.getState(), payload.short_id);
          if (sess) endSession(sess.id);
        })
      );

      // task_finished — update local task status, then close its session
      track(
        await listen<TaskFinishedEvent>(EVENT.TASK_FINISHED, ({ payload }) => {
          const s = useStore.getState();
          const existing = s.tasks.find((t) => t.short_id === payload.short_id);
          if (existing) s.upsertTask({ ...existing, status: payload.done_status });
          const sess = findSessionByTask(s, payload.short_id);
          if (sess) endSession(sess.id);
        })
      );

      // confirmation_requested — attribute to the task the backend named, never the active session.
      track(
        await listen<ConfirmationRequestedEvent>(EVENT.CONFIRMATION_REQUESTED, ({ payload }) => {
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
              .catch((e) => useStore.getState().setLastError(String(e)));
            return;
          }

          s.addConfirmation({
            id: payload.id,
            session_id: payload.session_id,
            op_type: payload.op_type,
            payload: payload.payload,
            origin: payload.origin,
          });
        })
      );

      // confirmation_resolved
      track(
        await listen<ConfirmationResolvedEvent>(EVENT.CONFIRMATION_RESOLVED, ({ payload }) => {
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
            invoke('push', { worktreeId: chainedPushWt }).catch((e) => s.setLastError(String(e)));
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
        })
      );

      // worktree_closed — patch the owning session's worktrees/repos
      track(
        await listen<WorktreeClosedEvent>(EVENT.WORKTREE_CLOSED, ({ payload }) => {
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
        })
      );

      // pty_started — route the new PTY into the session for its task
      track(
        await listen<PtyStartedEvent>(EVENT.PTY_STARTED, ({ payload }) => {
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
        })
      );

      // pty_output — dispatch to the session's xterm handler.
      track(
        await listen<PtyOutputEvent>(EVENT.PTY_OUTPUT, ({ payload }) =>
          deliverPtyOutput(payload.session_id, payload.b64))
      );

      // pty_exit — dispose the terminal host and drop the store row.
      track(
        await listen<PtyExitEvent>(EVENT.PTY_EXIT, ({ payload }) => {
          disposeHost(payload.session_id);
          const s = useStore.getState();
          const owner = findSessionByPty(s, payload.session_id);
          if (!owner) return;
          const pty = owner.ptySessions.find((p) => p.sessionId === payload.session_id);
          sessionActions(owner.id).removePtySession(payload.session_id);
          if (pty?.ptyType === 'agent' && owner.task) s.dropAgentActivity(owner.task.short_id);
        })
      );

      // backend_notice — a problem in work the user did not trigger.
      track(
        await listen<{
          kind: NotificationKind;
          source: NotificationSource;
          title: string;
          detail: string | null;
          task_id: string | null;
        }>(EVENT.BACKEND_NOTICE, ({ payload }) => {
          useStore.getState().notify({
            kind: payload.kind,
            source: payload.source,
            title: payload.title,
            detail: payload.detail ?? undefined,
            taskId: payload.task_id ?? undefined,
            goTo: payload.task_id ? { taskId: payload.task_id } : undefined,
          });
        })
      );

      // agent_activity — from Claude Code hooks. Toast only on the transition into
      // waiting, and only for an unfocused session.
      track(
        await listen<AgentActivity>(EVENT.AGENT_ACTIVITY, ({ payload }) => {
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
        })
      );

      // annotation_resolved — resolve it in whichever session holds it.
      track(
        await listen<{ id: string }>(EVENT.ANNOTATION_RESOLVED, ({ payload }) => {
          const s = useStore.getState();
          for (const id of s.sessionOrder) {
            if (s.sessions[id]?.annotations.some((a) => a.id === payload.id)) {
              sessionActions(id).resolveAnnotation(payload.id);
            }
          }
        })
      );

      // annotation_created — the agent left a note; `addAnnotation` dedupes by id.
      track(
        await listen<Annotation>(EVENT.ANNOTATION_CREATED, ({ payload }) => {
          const sess = findSessionByTask(useStore.getState(), payload.session_id);
          if (sess) sessionActions(sess.id).addAnnotation(payload);
        })
      );

      // annotation_updated — the agent rewrote a note's body.
      track(
        await listen<Annotation>(EVENT.ANNOTATION_UPDATED, ({ payload }) => {
          const sess = findSessionByTask(useStore.getState(), payload.session_id);
          if (sess) sessionActions(sess.id).updateAnnotation(payload.id, payload.content);
        })
      );

      // explorer_discarded — end the open session, PTYs included.
      track(
        await listen<{ short_id: string }>(EVENT.EXPLORER_DISCARDED, ({ payload }) => {
          const sess = findSessionByTask(useStore.getState(), payload.short_id);
          if (sess) endSession(sess.id);
        })
      );

      // rebase_conflict — a `rebase --continue` stopped on conflicts.
      track(
        await listen<RebaseConflictEvent>(EVENT.REBASE_CONFLICT, ({ payload }) => {
          const sess = findSessionByWorktree(useStore.getState(), payload.worktree_id);
          if (sess) applyRebaseConflict(sess.id, payload.worktree_id, payload.files ?? []);
        })
      );

      // rebase_done — the rebase finished or was aborted.
      track(
        await listen<RebaseDoneEvent>(EVENT.REBASE_DONE, ({ payload }) => {
          const s = useStore.getState();
          const sess = findSessionByWorktree(s, payload.worktree_id);
          if (!sess) return;
          sessionActions(sess.id).setRebaseConflict(null);
          s.notify({
            kind: 'success',
            source: 'git',
            taskId: sess.task?.short_id,
            title: payload.aborted ? 'Rebase aborted' : 'Rebase complete',
          });
          s.invalidateDiff(sess.id);
          s.refreshStatusFor(sess.id);
        })
      );

    };

    setup().catch(console.error);

    return () => {
      cancelled = true;
      unlisten.forEach((u) => u());
      unlisten.length = 0;
    };
  }, []);
}
