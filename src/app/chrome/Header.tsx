import { invoke } from '../../shared/ipc/invoke';
import { Layers, RefreshCw, ShieldAlert, Settings } from 'lucide-react';
import { useStore } from '../../shared/store';
import { NotificationCenter } from '../../notifications/NotificationCenter';
import { HeaderPickers } from '../../sessions/HeaderPickers';
import { WindowControls } from './WindowControls';

export function Header() {
  const syncStatus = useStore((s) => s.syncStatus);
  const setSyncStatus = useStore((s) => s.setSyncStatus);
  const setTasks = useStore((s) => s.setTasks);
  const setLastError = useStore((s) => s.setLastError);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);
  // Deferred approvals belong next to the other "waiting for you" counter, not
  // in the status bar where a parked agent write was easy to forget.
  const approvals = useStore((s) => s.pendingConfirmations.length);
  const setConfirmationsMinimized = useStore((s) => s.setConfirmationsMinimized);

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

        <HeaderPickers />
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
