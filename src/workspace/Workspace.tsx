import { useCallback, useRef } from 'react';
import { useSession, type LayoutNode } from '../shared/store';
import { containsLeaf } from '../shared/lib/layout';
import { WorkspacePane } from './WorkspacePane';
import { useAnnotations, type AnnCtx } from '../editor/useAnnotations';

export function Workspace() {
  const panes = useSession((s) => s.panes);
  const activePaneId = useSession((s) => s.activePaneId);
  const layout = useSession((s) => s.layout);
  const maximizedPaneId = useSession((s) => s.maximizedPaneId);
  const activeRepos = useSession((s) => s.activeRepos);
  const openTab = useSession((s) => s.openTab);
  const setSplitRatio = useSession((s) => s.setSplitRatio);

  // "Open in editor" from the diff inline form → an edit tab in the active pane.
  const openInEditor = useCallback((repoId: string, filePath: string, lineNum = 0) => {
    openTab({ repoId, filePath, view: 'edit', cursorLine: lineNum || undefined });
  }, [openTab]);
  const { ann } = useAnnotations(openInEditor);

  return (
    <div className="workspace-surface">
      <div className="ws-body">
        <LayoutView
          node={layout}
          panes={panes}
          activePaneId={activePaneId}
          maximizedPaneId={maximizedPaneId}
          multiRepo={activeRepos.length > 1}
          ann={ann}
          setSplitRatio={setSplitRatio}
        />
      </div>
    </div>
  );
}

interface LayoutViewProps {
  node: LayoutNode;
  panes: import('../shared/store').WorkspacePane[];
  activePaneId: string;
  maximizedPaneId: string | null;
  multiRepo: boolean;
  ann: AnnCtx;
  setSplitRatio: (splitId: string, ratio: number) => void;
}

/** Recursive split renderer: leaf → pane, split → two flex children + handle.
 *  Everything stays mounted; a maximized pane hides the rest via display. */
function LayoutView(props: LayoutViewProps) {
  const { node, panes, activePaneId, maximizedPaneId, multiRepo, ann } = props;

  if (node.kind === 'leaf') {
    const pane = panes.find((p) => p.id === node.paneId);
    if (!pane) return null;
    const hidden = maximizedPaneId !== null && maximizedPaneId !== pane.id;
    return (
      <div
        className="ws-pane-wrap"
        style={{ display: hidden ? 'none' : 'flex', flex: 1, minWidth: 0, minHeight: 0 }}
      >
        <WorkspacePane
          pane={pane}
          ann={ann}
          isActive={pane.id === activePaneId}
          multiRepo={multiRepo}
        />
      </div>
    );
  }

  return (
    <SplitView {...props} node={node} />
  );
}

function SplitView(props: LayoutViewProps & { node: Extract<LayoutNode, { kind: 'split' }> }) {
  const { node, maximizedPaneId, setSplitRatio } = props;
  const hostRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  // While a pane is maximized, only the subtree containing it is visible — the
  // other side (and the handle) hide, but everything stays mounted.
  const maximized = maximizedPaneId !== null;
  const showA = !maximized || containsLeaf(node.a, maximizedPaneId!);
  const showB = !maximized || containsLeaf(node.b, maximizedPaneId!);

  const startDrag = (e: React.MouseEvent) => {
    dragging.current = true;
    const dir = node.dir;
    document.body.style.cursor = dir === 'row' ? 'col-resize' : 'row-resize';
    document.body.style.userSelect = 'none';
    const onMove = (ev: MouseEvent) => {
      if (!dragging.current || !hostRef.current) return;
      const rect = hostRef.current.getBoundingClientRect();
      const ratio = dir === 'row'
        ? (ev.clientX - rect.left) / rect.width
        : (ev.clientY - rect.top) / rect.height;
      setSplitRatio(node.id, ratio);
    };
    const onUp = () => {
      dragging.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    e.preventDefault();
  };

  const childStyle = (show: boolean, ratio: number): React.CSSProperties =>
    show
      ? { flex: maximized ? '1 1 0%' : `${ratio} 1 0%`, minWidth: 0, minHeight: 0, display: 'flex' }
      : { display: 'none' };

  return (
    <div className={`ws-split ${node.dir}`} ref={hostRef}>
      <div className="ws-split-child" style={childStyle(showA, node.ratio)}>
        <LayoutView {...props} node={node.a} />
      </div>
      {showA && showB && (
        <div
          className={node.dir === 'row' ? 'resize-handle' : 'resize-handle-h'}
          onMouseDown={startDrag}
        />
      )}
      <div className="ws-split-child" style={childStyle(showB, 1 - node.ratio)}>
        <LayoutView {...props} node={node.b} />
      </div>
    </div>
  );
}
