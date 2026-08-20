import { invoke } from '@tauri-apps/api/core';
import { Layers, RefreshCw, PanelRight, ShieldAlert, Settings } from 'lucide-react';
import { useStore } from '../../shared/store';
import { NotificationCenter } from '../../notifications/NotificationCenter';
import { WindowControls } from './WindowControls';

export function Header() {
  const view = useStore((s) => s.view);
  const syncStatus = useStore((s) => s.syncStatus);
  const setSyncStatus = useStore((s) => s.setSyncStatus);
  const setTasks = useStore((s) => s.setTasks);
  const setLastError = useStore((s) => s.setLastError);
  const setView = useStore((s) => s.setView);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);
  // Deferred approvals belong next to the other "waiting for you" counter, not
  // in the status bar where a parked agent write was easy to forget.
  const approvals = useStore((s) => s.pendingConfirmations.length);
  const setConfirmationsMinimized = useStore((s) => s.setConfirmationsMinimized);
  const sessionCount = useStore((s) => s.sessionOrder.length);
  const dockOpen = useStore((s) => s.dockOpen);
  const setDockOpen = useStore((s) => s.setDockOpen);

  const handleSync = async () => {
    setSyncStatus('syncing');
    try {
      const tasks = await invoke<import('../../shared/ipc/ipc').Task[]>('list_tasks');
      setTasks(tasks);
      setSyncStatus('idle');
    } catch (e) {
      setSyncStatus('error');
      setLastError(String(e));
    }
  };

  return (
    <header className="header" data-tauri-drag-region>
      <div className="header-left" data-tauri-drag-region>
        <span className="header-logo" data-tauri-drag-region>
          <Layers size={14} strokeWidth={1.75} style={{ marginRight: 7, opacity: 0.7, verticalAlign: 'middle' }} />
          Groove
        </span>

        <button
          className={`header-back ${view === 'home' ? 'active' : ''}`}
          onClick={() => setView('home')}
        >
          Home
        </button>

      </div>
      <div className="header-right" data-tauri-drag-region>
        <button
          className="btn-sync"
          onClick={handleSync}
          disabled={syncStatus === 'syncing'}
          title="Sync tasks from Notion"
        >
          {syncStatus === 'syncing'
            ? <span className="btn-sync-spinner" />
            : <RefreshCw size={13} strokeWidth={2} />}
          {syncStatus === 'syncing' ? 'Syncing…' : 'Sync'}
        </button>
        {approvals > 0 && (
          <button
            className="header-approvals"
            onClick={() => setConfirmationsMinimized(false)}
            title={`${approvals} operation${approvals === 1 ? '' : 's'} awaiting your approval — click to review`}
          >
            <ShieldAlert size={13} strokeWidth={2} />
            {approvals}
          </button>
        )}
        <button
          className={`header-dock-btn ${dockOpen ? 'active' : ''}`}
          onClick={() => setDockOpen(!dockOpen)}
          title={`${sessionCount} open session${sessionCount === 1 ? '' : 's'} — show the dock (Alt+S)`}
        >
          <PanelRight size={14} strokeWidth={1.75} />
          {sessionCount > 0 && <span className="header-dock-count">{sessionCount}</span>}
        </button>
        <NotificationCenter />
        <button
          className="header-settings-btn"
          onClick={() => setSettingsOpen(true)}
          title="Settings"
        >
          <Settings size={14} strokeWidth={1.75} />
        </button>
        <WindowControls />
      </div>
    </header>
  );
}
