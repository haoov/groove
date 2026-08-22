import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, Copy, X } from 'lucide-react';

const appWindow = getCurrentWindow();

/** Custom minimize / maximize-restore / close controls for the frameless window. */
export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    appWindow.isMaximized().then(setMaximized).catch(() => {});
    appWindow
      .onResized(() => { appWindow.isMaximized().then(setMaximized).catch(() => {}); })
      .then((u) => { unlisten = u; })
      .catch(() => {});
    return () => { unlisten?.(); };
  }, []);

  return (
    <div className="window-controls">
      <button className="win-btn" title="Minimize" onClick={() => appWindow.minimize()}>
        <Minus size={15} strokeWidth={1.75} />
      </button>
      <button
        className="win-btn"
        title={maximized ? 'Restore' : 'Maximize'}
        onClick={() => appWindow.toggleMaximize()}
      >
        {maximized ? <Copy size={12} strokeWidth={1.75} /> : <Square size={12} strokeWidth={1.75} />}
      </button>
      <button className="win-btn win-btn-close" title="Close" onClick={() => appWindow.close()}>
        <X size={16} strokeWidth={1.75} />
      </button>
    </div>
  );
}
