import { useEffect } from 'react';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import { applyUiConfig } from '../shared/lib/ui';
import type { ConfigView } from '../shared/ipc/generated';
import { Header } from './chrome/Header';
import { Rail } from './chrome/Rail';
import { StatusBar } from './chrome/StatusBar';
import { Home } from '../home/Home';
import { SessionShell } from './SessionShell';
import { ApprovalModal } from '../approvals/ApprovalModal';
import { Toasts } from '../notifications/Toasts';
import { SettingsModal } from '../setup/SettingsModal';
import { FirstRun } from '../setup/FirstRun';
import { AddRepoModal } from '../setup/AddRepoModal';
import { CommandPalette } from '../command/CommandPalette';
import { useIpc } from './providers/useIpc';

export default function App() {
  useIpc();
  const config = useStore((s) => s.config);
  const setConfig = useStore((s) => s.setConfig);
  const view = useStore((s) => s.view);

  const setPaletteOpen = useStore((s) => s.setPaletteOpen);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);

  useEffect(() => {
    applyUiConfig(undefined); // Latte until the config arrives.
    call<ConfigView | null>('get_config')
      .then((c) => { setConfig(c); applyUiConfig(c?.ui); })
      .catch(() => setConfig(null));
  }, [setConfig]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); setPaletteOpen(true); }
      else if ((e.metaKey || e.ctrlKey) && e.key === ',') { e.preventDefault(); setSettingsOpen(true); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [setPaletteOpen, setSettingsOpen]);

  if (config === undefined) return <div className="booting" />;
  if (config === null) return <FirstRun onReady={setConfig} />;

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
      <SettingsModal />
      <AddRepoModal />
      <CommandPalette />
    </div>
  );
}
