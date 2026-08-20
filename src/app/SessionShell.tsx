import { useState } from 'react';
import { useStore } from '../shared/store';
import { Workspace } from '../workspace/Workspace';
import { Overview } from '../overview/Overview';
import { AgentConsole } from '../agent/AgentConsole';

/** The session frame: a mode body (code now, Overview in a later slice) beside a
 *  persistent agent console. The shell stays mounted across mode switches, so the
 *  agent keeps running while the user moves between Code and Overview. */
export function SessionShell() {
  const activeId = useStore((s) => s.activeSessionId);
  const session = useStore((s) => (activeId ? s.sessions[activeId] : undefined));
  const view = useStore((s) => s.view);
  const [agentOpen, setAgentOpen] = useState(true);

  if (!session) return <div className="placeholder">Opening session…</div>;

  const body = view === 'session' ? <Workspace /> : <Overview />;

  return (
    <div className={`session-shell${agentOpen ? '' : ' agent-hidden'}`}>
      <div className="session-body">{body}</div>
      {agentOpen ? (
        <AgentConsole key={session.id} sessionId={session.id} onCollapse={() => setAgentOpen(false)} />
      ) : (
        <button className="agent-reopen" title="Show agent" onClick={() => setAgentOpen(true)}>Agent ⟨</button>
      )}
    </div>
  );
}
