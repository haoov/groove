import { usePersisted, STRING_CODEC } from '../shared/lib/usePersisted';
import { useCallback, useMemo, useState } from 'react';
import {
  ChevronDown, ChevronRight, Plus, Minus, Circle, Trash2, GitCompare, Check,
} from 'lucide-react';
import { useSession, useStore } from '../shared/store';
import { useListNav } from '../shared/lib/useListNav';
import { ContextMenu } from '../shared/ui/ContextMenu';
import type { FileDiff } from '../shared/ipc/ipc';
import { guessLang } from '../shared/lib/lang';
import { StatBadge } from '../shared/ui/StatBadge';
import { repoDiffFor } from '../shared/lib/workspace';

/** Git status indicator: green + (added), yellow dot (modified), red − (deleted). */
function FileStatusIcon({ status }: { status: string }) {
  const st = status === 'A' || status === 'D' ? status : 'M';
  return (
    <span className={`changed-file-status st-${st}`}>
      {st === 'A' && <Plus size={13} strokeWidth={2.5} />}
      {st === 'D' && <Minus size={13} strokeWidth={2.5} />}
      {st === 'M' && <Circle size={7} strokeWidth={0} fill="currentColor" />}
    </span>
  );
}

const VIEW_KEY = 'wb.gitChangesView';

// One row list drives both views and the keyboard nav; "All changes" is row 0.
type Row =
  | { kind: 'all' }
  | { kind: 'dir'; path: string; depth: number; label: string; count: number }
  | { kind: 'file'; f: FileDiff; depth: number; name: string; parent: string };

function buildRows(files: FileDiff[], tree: boolean, collapsed: Set<string>): Row[] {
  const out: Row[] = [{ kind: 'all' }];
  if (!tree) {
    for (const f of files) {
      const name = f.path.split('/').pop() ?? f.path;
      const parent = f.path.slice(0, Math.max(0, f.path.length - name.length - 1));
      out.push({ kind: 'file', f, depth: 0, name, parent });
    }
    return out;
  }
  type Node = { dirs: Map<string, Node>; files: FileDiff[]; count: number };
  const root: Node = { dirs: new Map(), files: [], count: 0 };
  for (const f of files) {
    const segs = f.path.split('/');
    let node = root;
    for (let k = 0; k < segs.length - 1; k++) {
      let child = node.dirs.get(segs[k]);
      if (!child) { child = { dirs: new Map(), files: [], count: 0 }; node.dirs.set(segs[k], child); }
      child.count++;
      node = child;
    }
    node.files.push(f);
  }
  const walk = (node: Node, prefix: string, depth: number) => {
    for (const name of [...node.dirs.keys()].sort()) {
      const child = node.dirs.get(name)!;
      const path = prefix ? `${prefix}/${name}` : name;
      out.push({ kind: 'dir', path, depth, label: name, count: child.count });
      if (!collapsed.has(path)) walk(child, path, depth + 1);
    }
    for (const f of [...node.files].sort((a, b) => a.path.localeCompare(b.path))) {
      out.push({ kind: 'file', f, depth, name: f.path.split('/').pop() ?? f.path, parent: prefix });
    }
  };
  walk(root, '', 0);
  return out;
}

export function ChangedFilesList({
  repoId, worktreeId, onOpenFile, onOpenFileAlt, onOpenAll, onToggleStage, onDiscard, onStageAll, onDiscardAll,
}: {
  repoId: string | null;
  /** The active worktree of `repoId`. */
  worktreeId?: string;
  onOpenFile: (path: string, repoId: string, lang: string) => void;
  onOpenFileAlt: (path: string, repoId: string, lang: string) => void;
  /** Opens the whole repo's changes as one review tab. */
  onOpenAll: (repoId: string) => void;
  onToggleStage: (path: string, repoId: string, staged: boolean) => void;
  onDiscard: (path: string, repoId: string) => void;
  /** Stages or unstages every file. */
  onStageAll: (stage: boolean) => void;
  /** Discards every local change. */
  onDiscardAll: () => void;
}) {
  const diff = useSession((s) => s.diff);
  const panelFocusNonce = useStore((s) => s.panelFocusNonce);
  // Memoized: a fresh `[]` fallback per render would rebuild the row model.
  const files = useMemo(
    () => (repoId ? (repoDiffFor(diff, worktreeId, repoId)?.files ?? []) : []),
    [diff, worktreeId, repoId],
  );

  const [view, setView] = usePersisted(VIEW_KEY, 'list', STRING_CODEC);
  const treeView = view === 'tree';
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const showTree = useCallback((tree: boolean) => setView(tree ? 'tree' : 'list'), [setView]);
  const toggleDir = useCallback((path: string) => setCollapsed((s) => {
    const n = new Set(s);
    if (n.has(path)) n.delete(path); else n.add(path);
    return n;
  }), []);

  const totals = files.reduce(
    (acc, f) => ({ add: acc.add + f.added, del: acc.del + f.deleted }),
    { add: 0, del: 0 },
  );
  const stageable = files.filter((f) => f.staged != null);
  const anyUnstaged = stageable.some((f) => f.staged === false);
  const openAll = useCallback(() => { if (repoId) onOpenAll(repoId); }, [repoId, onOpenAll]);

  const rows = useMemo<Row[]>(() => buildRows(files, treeView, collapsed), [files, treeView, collapsed]);

  // Enter stages a file, toggles a folder, or opens the review on row 0.
  const onEnter = useCallback((i: number) => {
    const row = rows[i];
    if (!row || row.kind === 'all') return openAll();
    if (row.kind === 'dir') return toggleDir(row.path);
    const f = row.f;
    if (repoId && f.staged != null) onToggleStage(f.path, repoId, !(f.staged === true));
  }, [rows, repoId, onToggleStage, openAll, toggleDir]);
  const onRight = useCallback((i: number) => {
    const row = rows[i];
    if (!row || row.kind === 'all') return openAll();
    if (row.kind === 'dir') { setCollapsed((s) => { if (!s.has(row.path)) return s; const n = new Set(s); n.delete(row.path); return n; }); return; }
    if (repoId) onOpenFile(row.f.path, repoId, guessLang(row.f.path));
  }, [rows, repoId, onOpenFile, openAll]);
  const onLeft = useCallback((i: number): number | void => {
    const row = rows[i];
    if (!row || row.kind === 'all') return;
    if (row.kind === 'dir' && !collapsed.has(row.path)) return toggleDir(row.path);
    const parent = row.kind === 'dir' ? row.path.split('/').slice(0, -1).join('/') : row.parent;
    if (!parent) return;
    for (let k = i - 1; k >= 0; k--) { const r = rows[k]; if (r.kind === 'dir' && r.path === parent) return k; }
  }, [rows, collapsed, toggleDir]);
  const nav = useListNav({ count: rows.length, onEnter, onLeft, onRight, focusNonce: panelFocusNonce });

  if (!repoId) return <div className="sidebar-empty">Select a repo above</div>;
  if (files.length === 0) return <div className="sidebar-empty">No changed files</div>;

  const indent = (depth: number) => ({ paddingLeft: `calc(var(--space-3) + ${depth * 0.75}rem)` });

  return (
    <div
      className="files-list nav-list"
      tabIndex={0}
      ref={nav.containerRef}
      onKeyDown={nav.onKeyDown}
      onContextMenu={(e) => { e.preventDefault(); setMenu({ x: e.clientX, y: e.clientY }); }}
    >
      {rows.map((row, i) => {
        const selected = i === nav.index ? 'nav-selected' : '';
        if (row.kind === 'all') {
          return (
            <div key="__all__" className="changed-file-row">
              <button
                className={`changed-file changed-file-all ${selected}`}
                title="Open all of this repo's changes in one review tab"
                tabIndex={-1}
                onClick={() => { nav.setIndex(i); openAll(); }}
              >
                <GitCompare size={12} strokeWidth={1.75} className="changed-file-all-icon" />
                <span className="changed-file-name">All changes</span>
                <span className="changed-file-dir">{files.length} file{files.length === 1 ? '' : 's'}</span>
                <StatBadge stat={totals} />
              </button>
              {stageable.length > 0 && (
                <>
                  <input
                    type="checkbox"
                    className="changed-file-checkbox"
                    checked={!anyUnstaged}
                    title={anyUnstaged ? 'Stage all changes' : 'Unstage all changes'}
                    onChange={() => onStageAll(anyUnstaged)}
                  />
                  <button
                    className="changed-file-discard"
                    title="Discard all local changes"
                    onClick={(e) => { e.stopPropagation(); onDiscardAll(); }}
                  >
                    <Trash2 size={12} strokeWidth={1.75} />
                  </button>
                </>
              )}
            </div>
          );
        }
        if (row.kind === 'dir') {
          const open = !collapsed.has(row.path);
          return (
            <div key={`d:${row.path}`} className="changed-file-row">
              <button
                className={`changed-file changed-file-folder ${selected}`}
                style={indent(row.depth)}
                title={row.path}
                tabIndex={-1}
                onClick={() => { nav.setIndex(i); toggleDir(row.path); }}
              >
                {open ? <ChevronDown size={12} strokeWidth={2} /> : <ChevronRight size={12} strokeWidth={2} />}
                <span className="changed-file-name">{row.label}</span>
                <span className="changed-file-dir">{row.count}</span>
              </button>
            </div>
          );
        }
        const f = row.f;
        return (
          <div key={f.path} className="changed-file-row">
            <button
              className={`changed-file ${selected}`}
              style={treeView ? indent(row.depth) : undefined}
              title={f.path}
              tabIndex={-1}
              onClick={() => { nav.setIndex(i); onOpenFile(f.path, repoId, guessLang(f.path)); }}
              onDoubleClick={() => onOpenFileAlt(f.path, repoId, guessLang(f.path))}
            >
              <FileStatusIcon status={f.status} />
              <span className="changed-file-name">{row.name}</span>
              {!treeView && row.parent && <span className="changed-file-dir">{row.parent}</span>}
              <StatBadge stat={{ add: f.added, del: f.deleted }} />
            </button>
            {f.staged != null && (
              <>
                <input
                  type="checkbox"
                  className="changed-file-checkbox"
                  checked={f.staged === true}
                  title={f.staged ? 'Staged — click to unstage' : 'Stage this file'}
                  onChange={() => onToggleStage(f.path, repoId, !(f.staged === true))}
                />
                <button
                  className="changed-file-discard"
                  title="Discard changes to this file"
                  onClick={(e) => { e.stopPropagation(); onDiscard(f.path, repoId); }}
                >
                  <Trash2 size={12} strokeWidth={1.75} />
                </button>
              </>
            )}
          </div>
        );
      })}
      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)}>
          <button className="ctx-menu-item" onClick={() => { showTree(false); setMenu(null); }}>
            <Check size={13} style={{ opacity: treeView ? 0 : 1 }} /> Flat view
          </button>
          <button className="ctx-menu-item" onClick={() => { showTree(true); setMenu(null); }}>
            <Check size={13} style={{ opacity: treeView ? 1 : 0 }} /> Tree view
          </button>
        </ContextMenu>
      )}
    </div>
  );
}
