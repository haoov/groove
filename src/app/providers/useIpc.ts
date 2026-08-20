import { useEffect } from 'react';
import { on, EV } from '../../shared/ipc/events';
import { useStore } from '../../shared/store';
import type { WorkspacePayload } from '../../sessions/sessions.slice';
import type { Confirmation } from '../../approvals/approvals.slice';
import type { AgentActivity } from '../../shared/ipc/generated';

/** Wire backend events into the store. Mounted once at the app root. */
export function useIpc() {
  useEffect(() => {
    const store = useStore.getState;
    const offs: Promise<() => void>[] = [];

    offs.push(on<WorkspacePayload>(EV.workspaceReady, (p) => store().openWorkspace(p)));
    offs.push(
      on<{ task: { short_id: string; title: string }; kind: WorkspacePayload['kind'] }>(
        EV.workspaceStub,
        (p) => store().openWorkspace({ task: p.task as never, worktrees: [], repos: [], kind: p.kind }),
      ),
    );
    offs.push(on<{ short_id: string }>(EV.taskFinished, (p) => store().closeSession(p.short_id)));
    offs.push(on<{ short_id: string }>(EV.explorerDiscarded, (p) => store().closeSession(p.short_id)));

    // Approvals: the modal reads the queue; the store dedups by id.
    offs.push(on<Confirmation>(EV.confirmationRequested, (c) => store().pushConfirmation(c)));
    offs.push(on<{ id: string }>(EV.confirmationResolved, (p) => store().dropConfirmation(p.id)));

    // Notify only on the rising edge into "waiting on you", so one nudge per
    // block — not on every hook while the agent stays blocked.
    const waiting = new Set<string>();
    offs.push(
      on<AgentActivity>(EV.agentActivity, (a) => {
        if (a.state === 'waiting') {
          if (!waiting.has(a.task_id)) {
            waiting.add(a.task_id);
            store().notify('attention', `Agent needs you in ${a.task_id}`, a.task_id);
          }
        } else {
          waiting.delete(a.task_id);
        }
      }),
    );

    offs.push(
      on<{ kind?: string; message: string }>(EV.backendNotice, (n) => {
        const kind = n.kind === 'error' ? 'error' : n.kind === 'success' ? 'success' : 'info';
        store().notify(kind, n.message);
      }),
    );

    return () => {
      offs.forEach((p) => void p.then((off) => off()));
    };
  }, []);
}
