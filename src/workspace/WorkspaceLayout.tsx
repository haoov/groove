import { useState, useEffect, useRef } from 'react';
import { useWorkspaceData } from './useWorkspaceData';
import { useSession } from '../shared/store';
import { Sidebar } from './sidebar';
import { Workspace } from './Workspace';
import { OverviewView } from '../overview/OverviewView';

// Pixels, not a share of the row: a maximized agent content-sizes every level up
// to the panel column, and a percentage of a content-sized parent resolves to
// nothing — the column fell to its floor and the drag could not lift it.
const SIDEBAR_DEFAULT_PX = 300;
const SIDEBAR_MIN_PX = 176;
const SIDEBAR_MAX_PX = 640;
const SIDEBAR_WIDTH_KEY = 'wb.sidebarWidth';

const clampSidebar = (px: number) => Math.round(Math.max(SIDEBAR_MIN_PX, Math.min(SIDEBAR_MAX_PX, px)));

function readSidebarWidth(): number {
  try {
    const saved = Number(localStorage.getItem(SIDEBAR_WIDTH_KEY));
    return Number.isFinite(saved) && saved > 0 ? clampSidebar(saved) : SIDEBAR_DEFAULT_PX;
  } catch {
    return SIDEBAR_DEFAULT_PX;
  }
}

/** One session's workspace, by mode: the Overview page (full width, no sidebar
 *  band), or sidebar + the recursive tab/pane surface. Agent and terminal live
 *  as tabs inside the surface (see lib/panes.ts conventions). */
export function WorkspaceLayout() {
  useWorkspaceData();

  const mode = useSession((s) => s.workspaceMode);
  // Collapsed by its own shortcut (pressing panel.* while already focused there).
  const collapsed = useSession((s) => s.sidebarCollapsed);

  const [sidebarPx, setSidebarPx] = useState(readSidebarWidth);

  const wrapRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  const startX = useRef(0);
  const startSize = useRef(0);

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (!dragging.current) return;
      const delta = e.clientX - startX.current;
      setSidebarPx(clampSidebar(startSize.current + delta));
    };
    const onUp = () => {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      const w = wrapRef.current?.getBoundingClientRect().width;
      if (w) { try { localStorage.setItem(SIDEBAR_WIDTH_KEY, String(Math.round(w))); } catch { /* ignore */ } }
    };
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    return () => {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };
  }, []);

  const startSidebarDrag = (e: React.MouseEvent) => {
    dragging.current = true;
    startX.current = e.clientX;
    // The rendered width, not the stored one: in a window too narrow for every
    // column this has been shrunk, and dragging from the stored value would jump.
    startSize.current = wrapRef.current?.getBoundingClientRect().width ?? SIDEBAR_MIN_PX;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    e.preventDefault();
  };

  if (mode === 'overview') {
    return (
      <div className="workspace">
        <div className="workspace-main">
          <OverviewView />
        </div>
      </div>
    );
  }

  return (
    <div className="workspace">
      {!collapsed && (
        <>
          <div className="sidebar-wrapper" ref={wrapRef} style={{ width: sidebarPx, minWidth: SIDEBAR_MIN_PX }}>
            <Sidebar />
          </div>
          <div className="resize-handle" onMouseDown={startSidebarDrag} />
        </>
      )}
      <div className="workspace-main">
        <Workspace />
      </div>
    </div>
  );
}
