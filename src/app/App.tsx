import { useEffect } from 'react';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import type { ConfigView } from '../shared/ipc/generated';
import { Header } from './chrome/Header';
import { Rail } from './chrome/Rail';
import { StatusBar } from './chrome/StatusBar';
import { Home } from '../home/Home';
import { SessionShell } from './SessionShell';
import { ApprovalModal } from '../approvals/ApprovalModal';
import { Toasts } from '../notifications/Toasts';
import { useIpc } from './providers/useIpc';

export default function App() {
  useIpc();
  const config = useStore((s) => s.config);
  const setConfig = useStore((s) => s.setConfig);
  const view = useStore((s) => s.view);

  useEffect(() => {
    // The mockup's identity is Catppuccin Latte; real theme selection lands with Settings.
    document.documentElement.dataset.theme = 'latte';
    call<ConfigView | null>('get_config')
      .then((c) => setConfig(c))
      .catch(() => setConfig(null));
  }, [setConfig]);

  if (config === undefined) return <div className="booting" />;
  if (config === null) {
    return (
      <div className="setup">
        <div>
          <h1>Groove isn't set up yet</h1>
          <p>First-run configuration lands in the setup slice.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      <Header />
      <div className="stage">
        <Rail />
        <main className="main">
          {view === 'home' ? <Home /> : <SessionShell />}
        </main>
      </div>
      <StatusBar />
      <ApprovalModal />
      <Toasts />
    </div>
  );
}
