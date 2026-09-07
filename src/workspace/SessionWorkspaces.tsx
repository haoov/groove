import { useStore, SessionIdContext } from '../shared/store';
import { WorkspaceLayout } from './WorkspaceLayout';

/**
 * Hosts every open session's workspace at once. Inactive ones stay mounted but hidden,
 * so terminals, tabs and scroll state survive switching.
 */
export function SessionWorkspaces({ hidden }: { hidden: boolean }) {
  const allSessions = useStore((s) => s.sessionOrder);
  const activeSessionId = useStore((s) => s.activeSessionId);
  // With the agent maximized the column must be content-sized: `flex: 1` resolves it to zero width.
  const agentMaximized = useStore((s) => s.agentMaximized);
  const grow = agentMaximized ? ('0 1 auto' as const) : 1;
  const sessionOrder = allSessions;

  if (sessionOrder.length === 0) return null;

  return (
    <div
      className="session-workspaces"
      style={{ display: hidden ? 'none' : 'flex', flex: grow, minWidth: 0, minHeight: 0 }}
    >
      {sessionOrder.map((id) => {
        const active = id === activeSessionId;
        return (
          <div
            key={id}
            className="session-host"
            style={{ display: active ? 'flex' : 'none', flex: grow, minWidth: 0, minHeight: 0 }}
          >
            <SessionIdContext.Provider value={id}>
              <WorkspaceLayout />
            </SessionIdContext.Provider>
          </div>
        );
      })}
    </div>
  );
}
