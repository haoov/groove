import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// Frameless windows (decorations: false) lose the OS edge/corner resize grips —
// especially on Linux/GTK — so we overlay our own thin handles, like Zed/VSCode.
// `ResizeDirection` isn't exported from the api package; its runtime values are
// these strings, which `startResizeDragging` forwards to the backend as-is.
const HANDLES: [string, string][] = [
  ['n', 'North'],
  ['s', 'South'],
  ['e', 'East'],
  ['w', 'West'],
  ['nw', 'NorthWest'],
  ['ne', 'NorthEast'],
  ['sw', 'SouthWest'],
  ['se', 'SouthEast'],
];

export function ResizeHandles() {
  return (
    <>
      {HANDLES.map(([cls, dir]) => (
        <div
          key={cls}
          className={`resize-grip resize-grip-${cls}`}
          onMouseDown={(e) => {
            if (e.button !== 0) return;
            e.preventDefault();
            appWindow.startResizeDragging(dir as any).catch(() => {});
          }}
        />
      ))}
    </>
  );
}
