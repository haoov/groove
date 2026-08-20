import { useStore } from '../../shared/store';
import { Icon, type IconName } from '../../shared/ui';
import type { SidebarPanel } from '../../sessions/sessions.slice';

export function Rail() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const activeId = useStore((s) => s.activeSessionId);
  const session = useStore((s) => (activeId ? s.sessions[activeId] : undefined));
  const setSidebar = useStore((s) => s.setSidebar);

  const inSession = view !== 'home';
  const panel: [SidebarPanel, IconName][] = [
    ['files', 'files'],
    ['git', 'git'],
    ['notes', 'notes'],
  ];
  const pickPanel = (p: SidebarPanel) => {
    if (session) setSidebar(session.id, p);
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
          {panel.map(([p, icon]) => (
            <button
              key={p}
              className={`r${view === 'session' && session?.sidebar === p ? ' on' : ''}`}
              title={p[0].toUpperCase() + p.slice(1)}
              onClick={() => pickPanel(p)}
            >
              <Icon name={icon} />
            </button>
          ))}
        </div>
      )}
      <button className="r r-bottom" title="Command palette"><Icon name="cmd" /></button>
    </nav>
  );
}
