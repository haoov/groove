import { useEffect } from 'react';
import { on, EV } from '../../shared/ipc/events';
import { useStore } from '../../shared/store';
import type { WorkspacePayload } from '../../sessions/sessions.slice';

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

    return () => {
      offs.forEach((p) => void p.then((off) => off()));
    };
  }, []);
}
