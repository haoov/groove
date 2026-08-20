import { useStore } from '../../shared/store';
import { call } from '../../shared/ipc/client';
import { Button, Icon } from '../../shared/ui';
import { Pickers } from '../../sessions/Pickers';

/** The top context bar: Home nav + New on the dashboard; session context pickers
 *  inside a session. */
export function Header() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const setActiveSession = useStore((s) => s.setActiveSession);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);

  const newExplorer = async () => {
    try {
      const id = await call<string>('open_explorer_session', {});
      setActiveSession(id);
      setView('session');
    } catch (e) {
      console.warn('open_explorer_session failed', e);
    }
  };

  return (
    <header className="topbar">
      <div className="wordmark"><span className="knob" />groove</div>
      {view === 'home' ? (
        <>
          <button className="navpill" onClick={() => setView('home')}>Home</button>
          <div className="spring" />
          <Button variant="accent" onClick={newExplorer} title="Open a scratch explorer session">+ New explorer</Button>
        </>
      ) : (
        <>
          <Pickers />
          <div className="spring" />
        </>
      )}
      <div style={{ display: 'flex', gap: 2 }}>
        <button className="iconbtn" title="Notifications"><Icon name="bell" /></button>
        <button className="iconbtn" title="Preferences" onClick={() => setSettingsOpen(true)}><Icon name="settings" /></button>
      </div>
    </header>
  );
}
