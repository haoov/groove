import { LayoutGrid, LayoutList, Files, GitBranch, MessageSquare, Command } from 'lucide-react';
import { useStore, useSession, type SidebarTab } from '../../shared/store';

/**
 * Far-left activity rail (Zed/VS Code style). Owns top-level navigation
 * (Home) and, when a session is open, the sidebar panel switch
 * (Files / Git / Annotations) that used to live as tabs inside the sidebar.
 */
export function ActivityRail() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const reviewCount = useStore((s) => s.reviewQueue?.length ?? 0);
  const activeTask = useSession((s) => s.activeTask);
  const sidebarTab = useSession((s) => s.sidebarTab);
  const setSidebarTab = useSession((s) => s.setSidebarTab);
  const openTab = useSession((s) => s.openTab);
  const annotations = useSession((s) => s.annotations);
  const setCommandPaletteOpen = useStore((s) => s.setCommandPaletteOpen);

  const openCount = annotations.filter((a) => a.status === 'open').length;
  const inWorkspace = view === 'workspace';
  // The Files/Git/Annotations panel applies to any real workspace session
  // (task / explorer / review) — all have a non-null active task.
  const showPanels = !!activeTask;

  const selectPanel = (tab: SidebarTab) => {
    setSidebarTab(tab);
    setView('workspace');
  };

  const openOverview = () => {
    openTab({ repoId: '', filePath: '', view: 'diff', kind: 'overview' });
    setView('workspace');
  };

  return (
    <nav className="activity-rail">
      {/* Home: reviews strip / tasks kanban / explorers strip. */}
      <div className="rail-group">
        <button
          className={`rail-btn ${!inWorkspace ? 'active' : ''}`}
          onClick={() => setView('home')}
          title={reviewCount > 0
            ? `Home — ${reviewCount} review${reviewCount === 1 ? '' : 's'} waiting on you`
            : 'Home'}
        >
          <LayoutGrid size={18} strokeWidth={1.75} />
          {reviewCount > 0 && <span className="rail-badge">{reviewCount}</span>}
        </button>
      </div>

      {/* Open (or refocus) the session overview tab. */}
      {showPanels && (
        <div className="rail-group">
          <button
            className="rail-btn"
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
            className={`rail-btn ${inWorkspace && sidebarTab === 'files' ? 'active' : ''}`}
            onClick={() => selectPanel('files')}
            title="Files"
          >
            <Files size={18} strokeWidth={1.75} />
          </button>
          <button
            className={`rail-btn ${inWorkspace && sidebarTab === 'git' ? 'active' : ''}`}
            onClick={() => selectPanel('git')}
            title="Source control"
          >
            <GitBranch size={18} strokeWidth={1.75} />
          </button>
          <button
            className={`rail-btn ${inWorkspace && sidebarTab === 'annotations' ? 'active' : ''}`}
            onClick={() => selectPanel('annotations')}
            title="Annotations"
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
