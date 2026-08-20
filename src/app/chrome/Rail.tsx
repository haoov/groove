import { useStore } from '../../shared/store';
import { Icon } from '../../shared/ui';
import type { SidebarPanel } from '../../sessions/sessions.slice';

export function Rail() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const activeId = useStore((s) => s.activeSessionId);
  const session = useStore((s) => (activeId ? s.sessions[activeId] : undefined));
  const setSidebar = useStore((s) => s.setSidebar);
  const openChangesTab = useStore((s) => s.openChangesTab);

  const inSession = view !== 'home';
  const activeChanges = view === 'session' && session?.activeTabId === 'changes';
  // Files and Notes are sidebar panels; Git opens the diff review as a tab
  // (a full Git sidebar panel lands in a later slice).
  const pickPanel = (p: SidebarPanel) => {
    if (session) setSidebar(session.id, p);
    setView('session');
  };
  const openGit = () => {
    if (session) openChangesTab(session.id);
    setView('session');
  };

  return (
    <nav className="rail">
      <div className="rgroup">
        <button className={`r${view === 'home' ? ' on' : ''}`} title="Home" onClick={() => setView('home')}>
          <Icon name="home" />
        </button>
        {view === 'home' && (
          <button className="r" title="Reviews"><Icon name="reviews" /><i className="num">3</i></button>
        )}
      </div>
      {inSession && (
        <div className="rgroup">
          <button
            className={`r${view === 'overview' || view === 'review' ? ' on' : ''}`}
            title="Overview"
            onClick={() => setView(session?.kind === 'review' ? 'review' : 'overview')}
          >
            <Icon name="overview" />
          </button>
          <button
            className={`r${view === 'session' && session?.sidebar === 'files' ? ' on' : ''}`}
            title="Files"
            onClick={() => pickPanel('files')}
          >
            <Icon name="files" />
          </button>
          <button
            className={`r${activeChanges ? ' on' : ''}`}
            title="Changes"
            onClick={openGit}
          >
            <Icon name="git" />
          </button>
          <button
            className={`r${view === 'session' && session?.sidebar === 'notes' ? ' on' : ''}`}
            title="Notes"
            onClick={() => pickPanel('notes')}
          >
            <Icon name="notes" />
          </button>
        </div>
      )}
      <button className="r r-bottom" title="Command palette"><Icon name="cmd" /></button>
    </nav>
  );
}
