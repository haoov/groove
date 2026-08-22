import { LayoutGrid, LayoutList, Files, GitBranch, MessageSquare, Command } from 'lucide-react';
import { useStore, useSession, type SidebarTab } from '../../shared/store';

/**
 * Far-left activity rail (Zed/VS Code style). Owns top-level navigation
 * (Home) and, when a session is open, the sidebar panel switch
 * (Files / Git / Notes) that used to live as tabs inside the sidebar.
 */
export function ActivityRail() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const activeTask = useSession((s) => s.activeTask);
  const sidebarTab = useSession((s) => s.sidebarTab);
  const setSidebarTab = useSession((s) => s.setSidebarTab);
  const sidebarCollapsed = useSession((s) => s.sidebarCollapsed);
  const setSidebarCollapsed = useSession((s) => s.setSidebarCollapsed);
  const workspaceMode = useSession((s) => s.workspaceMode);
  const setWorkspaceMode = useSession((s) => s.setWorkspaceMode);
  const annotations = useSession((s) => s.annotations);
  const setCommandPaletteOpen = useStore((s) => s.setCommandPaletteOpen);

  const openCount = annotations.filter((a) => a.status === 'open').length;
  const inWorkspace = view === 'workspace';
  // The Files/Git/Notes panel applies to any real workspace session
  // (task / explorer / review) — all have a non-null active task.
  const showPanels = !!activeTask;

  const selectPanel = (tab: SidebarTab) => {
    // Clicking the panel you are already on folds it away, like Zed's dock icons.
    const here = inWorkspace && workspaceMode === 'code' && sidebarTab === tab && !sidebarCollapsed;
    if (here) {
      setSidebarCollapsed(true);
      return;
    }
    setSidebarTab(tab);
    setSidebarCollapsed(false);
    setWorkspaceMode('code');
    setView('workspace');
  };

  const openOverview = () => {
    setWorkspaceMode('overview');
    setView('workspace');
  };

  return (
    <nav className="activity-rail">
      {/* Home: Live · Up next · Reviews tabs (the review count lives in the tab). */}
      <div className="rail-group">
        <button
          className={`rail-btn ${!inWorkspace ? 'active' : ''}`}
          onClick={() => setView('home')}
          title="Home"
        >
          <LayoutGrid size={18} strokeWidth={1.75} />
        </button>
      </div>

      {/* The session's Overview MODE — the ticket / explorer / MR page. */}
      {showPanels && (
        <div className="rail-group">
          <button
            className={`rail-btn ${inWorkspace && workspaceMode === 'overview' ? 'active' : ''}`}
            onClick={openOverview}
            title="Overview"
          >
            <LayoutList size={18} strokeWidth={1.75} />
          </button>
        </div>
      )}

      {showPanels && (
        <div className="rail-group">
          <button
            className={`rail-btn ${inWorkspace && workspaceMode === 'code' && !sidebarCollapsed && sidebarTab === 'files' ? 'active' : ''}`}
            onClick={() => selectPanel('files')}
            title="Files"
          >
            <Files size={18} strokeWidth={1.75} />
          </button>
          <button
            className={`rail-btn ${inWorkspace && workspaceMode === 'code' && !sidebarCollapsed && sidebarTab === 'git' ? 'active' : ''}`}
            onClick={() => selectPanel('git')}
            title="Source control"
          >
            <GitBranch size={18} strokeWidth={1.75} />
          </button>
          <button
            className={`rail-btn ${inWorkspace && workspaceMode === 'code' && !sidebarCollapsed && sidebarTab === 'annotations' ? 'active' : ''}`}
            onClick={() => selectPanel('annotations')}
            title="Notes"
          >
            <MessageSquare size={18} strokeWidth={1.75} />
            {openCount > 0 && <span className="rail-badge">{openCount}</span>}
          </button>
        </div>
      )}

      <button
        className="rail-btn rail-btn-bottom"
        onClick={() => setCommandPaletteOpen(true)}
        title="Command palette (⌘K)"
      >
        <Command size={18} strokeWidth={1.75} />
      </button>
    </nav>
  );
}
