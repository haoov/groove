import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Terminal } from 'lucide-react';
import { useStore, useSession } from '../store';
import { activeWorktreeFor } from '../lib/workspace';
import { toggleTerminal } from '../lib/panes';

export function StatusBar() {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const worktreeStatus = useSession((s) => s.worktreeStatus);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);
  const panes = useSession((s) => s.panes);
  const syncStatus = useStore((s) => s.syncStatus);
  const lastError = useStore((s) => s.lastError);
  const setNotificationsOpen = useStore((s) => s.setNotificationsOpen);

  // Read the endpoint from the server that owns it rather than restating it.
  const [mcpEndpoint, setMcpEndpoint] = useState<string | null>(null);
  useEffect(() => {
    invoke<string>('get_mcp_endpoint').then(setMcpEndpoint).catch(() => setMcpEndpoint(null));
  }, []);

  const terminalTabOpen = panes.some((p) => p.tabs.some((t) => t.kind === 'terminal'));

  return (
    <footer className="statusbar">
      <div className="statusbar-left">
        {activeTask ? (
          <span className="statusbar-task">
            <span className="statusbar-dot active" />
            {activeTask.short_id}
          </span>
        ) : (
          <span className="statusbar-idle">No active task</span>
        )}

        {/* At-a-glance per-repo git status (replaces the old sidebar chip dashboard). */}
        {activeTask && activeRepos.map((repo) => {
          const wt = activeWorktreeFor(activeWorktrees, repo.id);
          const st = wt ? worktreeStatus[wt.id] : undefined;
          const dirty = st ? st.modified + st.staged : 0;
          const gone = st?.remote_branch_gone ?? false;
          const title = st
            ? `${repo.project}: ${st.modified} modified, ${st.staged} staged, ↑${st.ahead} ↓${st.behind}${gone ? ' · remote branch gone' : ''}`
            : repo.project;
          return (
            <button
              key={repo.id}
              className={`statusbar-repo ${gone ? 'gone' : ''}`}
              title={title}
              onClick={() => setActiveRepoId(repo.id)}
            >
              <span className={`statusbar-dot ${dirty > 0 ? 'dirty' : st ? 'clean' : ''}`} />
              {repo.project}
              {dirty > 0 && <span className="statusbar-repo-dirty">{dirty}</span>}
              {st && st.ahead > 0 && <span className="statusbar-repo-ahead">↑{st.ahead}</span>}
              {st && st.behind > 0 && <span className="statusbar-repo-behind">↓{st.behind}</span>}
            </button>
          );
        })}
      </div>
      <div className="statusbar-right">
        {/* "Something is wrong right now" — clicking opens the feed, where the
            full text and every earlier error still are. */}
        {lastError && (
          <button
            className="statusbar-error"
            title={`${lastError}\n\nClick to open notifications`}
            onClick={() => setNotificationsOpen(true)}
          >
            ⚠ Error
          </button>
        )}
        {syncStatus === 'syncing' && (
          <span className="statusbar-syncing">Syncing…</span>
        )}

        {/* Terminal toggle. The agent has no button here: its console carries its
            own always-visible status bar. */}
        {activeTask && (
          <span className="statusbar-docks">
            <button
              className={`statusbar-dock-btn ${terminalTabOpen ? 'active' : ''}`}
              onClick={toggleTerminal}
              title="Terminal (Alt+`)"
            >
              <Terminal size={12} strokeWidth={1.75} />
            </button>
          </span>
        )}
        {syncStatus === 'idle' && !lastError && mcpEndpoint && (
          <span className="statusbar-ok" title={`MCP server on ${mcpEndpoint}`}>
            MCP :{mcpEndpoint.split(':').pop()}
          </span>
        )}
      </div>
    </footer>
  );
}
