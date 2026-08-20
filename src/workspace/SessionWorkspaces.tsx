import { useStore, SessionIdContext } from '../shared/store';
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
  // A maximized agent hides the pane area (styles/console.css), leaving this column
  // holding nothing but the panel — so it has to be as wide as the panel and no
  // wider. `flex: 1` cannot express that: its zero basis, with min-width 0, gives
  // the panel's own width nothing to travel up through, and the column resolved to
  // nothing instead. Content-sized in that mode, and still shrinkable.
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
