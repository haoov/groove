import { useState, useEffect, useRef } from 'react';
import { useWorkspaceData } from './useWorkspaceData';
import { useSession } from '../shared/store';
import { Sidebar } from './sidebar';
import { Workspace } from './Workspace';
import { OverviewView } from '../overview/OverviewView';

// Width is a percentage of the workspace, with a rem floor so it stays usable.
const SIDEBAR_DEFAULT_PCT = 18;
const SIDEBAR_MIN_PCT = 12;
const SIDEBAR_MAX_PCT = 40;
const SIDEBAR_MIN_PX = 176; // ~11rem — hard floor for the drag

/** One session's workspace, by mode: the Overview page (full width, no sidebar
 *  band), or sidebar + the recursive tab/pane surface. Agent and terminal live
 *  as tabs inside the surface (see lib/panes.ts conventions). */
export function WorkspaceLayout() {
  useWorkspaceData();

  const mode = useSession((s) => s.workspaceMode);
  // Collapsed by its own shortcut (pressing panel.* while already focused there).
  const collapsed = useSession((s) => s.sidebarCollapsed);

  const [sidebarPct, setSidebarPct] = useState(SIDEBAR_DEFAULT_PCT);

  const wrapRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  const startX = useRef(0);
  const startSize = useRef(0);

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (!dragging.current) return;
      const delta = e.clientX - startX.current;
      // Convert the pixel drag to a percentage of the workspace row.
      const container = wrapRef.current?.parentElement?.getBoundingClientRect().width ?? window.innerWidth;
      const px = Math.max(SIDEBAR_MIN_PX, startSize.current + delta);
      const pct = (px / container) * 100;
      setSidebarPct(Math.max(SIDEBAR_MIN_PCT, Math.min(SIDEBAR_MAX_PCT, pct)));
    };
    const onUp = () => {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
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
          <div className="sidebar-wrapper" ref={wrapRef} style={{ width: `${sidebarPct}%`, minWidth: `${SIDEBAR_MIN_PX}px` }}>
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
