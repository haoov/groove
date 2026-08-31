import { useEffect, useState } from 'react';
import { invoke } from '../../shared/ipc/invoke';
import { useStore } from '../../shared/store';

export function StatusBar() {
  // The active session's title (task title, or MR name for reviews); null off-workspace.
  const title = useStore((s) => {
    if (s.view !== 'workspace') return null;
    const sess = s.activeSessionId ? s.sessions[s.activeSessionId] : null;
    return sess ? (sess.task?.title || sess.title || null) : null;
  });
  const syncStatus = useStore((s) => s.syncStatus);

  // Read the endpoint from the server that owns it rather than restating it.
  const [mcpEndpoint, setMcpEndpoint] = useState<string | null>(null);
  useEffect(() => {
    invoke<string>('get_mcp_endpoint').then(setMcpEndpoint).catch(() => setMcpEndpoint(null));
  }, []);

  return (
    <footer className="statusbar">
      <div className="statusbar-left" />
      <div className="statusbar-center">
        {title ? (
          <span className="statusbar-task" title={title}>
            <span className="statusbar-dot active" />
            <span className="statusbar-task-text">{title}</span>
          </span>
        ) : (
          <span className="statusbar-idle">No active task</span>
        )}
      </div>
      <div className="statusbar-right">
        {syncStatus === 'syncing' && (
          <span className="statusbar-syncing">Syncing…</span>
        )}
        {syncStatus === 'idle' && mcpEndpoint && (
          <span className="statusbar-ok" title={`MCP server on ${mcpEndpoint}`}>
            MCP :{mcpEndpoint.split(':').pop()}
          </span>
        )}
      </div>
    </footer>
  );
}
