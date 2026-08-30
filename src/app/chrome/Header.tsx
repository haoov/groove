import { ShieldAlert, Settings } from 'lucide-react';
import { useStore } from '../../shared/store';
import { NotificationCenter } from '../../notifications/NotificationCenter';
import { HeaderPickers } from '../../sessions/HeaderPickers';
import { WindowControls } from './WindowControls';
import { isMac } from '../../shared/lib/platform';
import { GrooveMark } from '../../shared/ui/GrooveMark';

export function Header() {
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);
  // Deferred approvals belong next to the other "waiting for you" counter, not
  // in the status bar where a parked agent write was easy to forget.
  const approvals = useStore((s) => s.pendingConfirmations.length);
  const setConfirmationsMinimized = useStore((s) => s.setConfirmationsMinimized);

  return (
    <header className="header" data-tauri-drag-region>
      <div className="header-left" data-tauri-drag-region>
        <span className="header-logo" data-tauri-drag-region>
          <GrooveMark size={16} />
          roove
        </span>

        <HeaderPickers />
      </div>

      <div className="header-center" data-tauri-drag-region />

      <div className="header-right" data-tauri-drag-region>
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
        {/* macOS has its own traffic lights. */}
        {!isMac() && <WindowControls />}
      </div>
    </header>
  );
}
