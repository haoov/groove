import { useStore, SessionIdContext } from '../store';
import { useDeskId } from '../lib/desk';
import { WorkspaceLayout } from './WorkspaceLayout';

/**
 * Hosts every open session's workspace at once, showing only the active one
 * (the rest are kept mounted but hidden via `display:none`). Keeping them
 * mounted is what lets each session's agent/terminal xterm — and its tabs,
 * diff, and scroll state — survive switching away. Stays mounted even while
 * Home is showing so popping back to a session never loses its terminal.
 */
export function SessionWorkspaces({ hidden }: { hidden: boolean }) {
  const allSessions = useStore((s) => s.sessionOrder);
  const activeSessionId = useStore((s) => s.activeSessionId);
  // The desk is not a workspace: mounting one would build an editor, sidebar and
  // overview for a session that has no repos and is never focused.
  const deskId = useDeskId();
  const sessionOrder = allSessions.filter((id) => id !== deskId);

  if (sessionOrder.length === 0) return null;

  return (
    <div
      className="session-workspaces"
      style={{ display: hidden ? 'none' : 'flex', flex: 1, minWidth: 0, minHeight: 0 }}
    >
      {sessionOrder.map((id) => {
        const active = id === activeSessionId;
        return (
          <div
            key={id}
            className="session-host"
            style={{ display: active ? 'flex' : 'none', flex: 1, minWidth: 0, minHeight: 0 }}
          >
            <SessionIdContext.Provider value={id}>
              {/* Every kind (task / explorer / review) is a full workspace over
                  its synthetic-or-real task. */}
              <WorkspaceLayout />
            </SessionIdContext.Provider>
          </div>
        );
      })}
    </div>
  );
}
