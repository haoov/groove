import { useStore } from '../../shared/store';
import { call } from '../../shared/ipc/client';
import { Button, Icon } from '../../shared/ui';

/** The top context bar. On Home it carries the Home nav + New explorer; the
 *  session context pickers arrive with the sessions slice. */
export function Header() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const setActiveSession = useStore((s) => s.setActiveSession);

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
      {view === 'home' && (
        <>
          <button className="navpill" onClick={() => setView('home')}>Home</button>
          <div className="spring" />
          <Button variant="accent" onClick={newExplorer} title="Open a scratch explorer session">+ New explorer</Button>
        </>
      )}
      {view !== 'home' && <div className="spring" />}
      <div style={{ display: 'flex', gap: 2 }}>
        <button className="iconbtn" title="Notifications"><Icon name="bell" /></button>
        <button className="iconbtn" title="Settings"><Icon name="settings" /></button>
      </div>
    </header>
  );
}
