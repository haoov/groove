import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// Resize grips for the frameless window.
// `ResizeDirection` is not exported from the api package; these strings are its runtime values.
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
