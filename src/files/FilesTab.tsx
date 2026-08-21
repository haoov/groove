import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { Search, TextSearch, X, ChevronRight, ChevronDown } from 'lucide-react';
import { useSession, useStore } from '../shared/store';
import { useListNav } from '../shared/lib/useListNav';
import { guessLang } from '../shared/lib/lang';
import { matchRanges, Highlighted } from '../shared/lib/match';
import type { Worktree, SearchMatch } from '../shared/ipc/ipc';
import { buildTree, flattenVisible, FileTreeNodes, FileTypeIcon, type TreeNode } from './tree';
import {
  TreeContextMenu, TreePrompt, TreeConfirmDelete, type MenuAction, type TreeClipboard,
} from './FileTreeMenu';

const PREVIEW_DEBOUNCE_MS = 80;
const GREP_DEBOUNCE_MS = 160;
const MAX_RESULTS = 50;

type SearchMode = 'name' | 'text';
/** One file's matches. The list is rendered VS Code style — the file as a header
 *  with its hits indented under it — so every match is individually reachable
 *  instead of only the first one in each file. */
interface GrepFile { file: string; matches: SearchMatch[] }

/** The flattened list the keyboard walks: a file header, then its matches. */
type GrepRow =
  | { kind: 'file'; file: GrepFile }
  | { kind: 'match'; file: GrepFile; match: SearchMatch };

function grepRows(files: GrepFile[], collapsed: Set<string>): GrepRow[] {
  const rows: GrepRow[] = [];
  for (const file of files) {
    rows.push({ kind: 'file', file });
    if (collapsed.has(file.file)) continue;
    for (const match of file.matches) rows.push({ kind: 'match', file, match });
  }
  return rows;
}

function splitPath(path: string): { name: string; dir: string } {
  const idx = path.lastIndexOf('/');
  return idx === -1 ? { name: path, dir: '' } : { name: path.slice(idx + 1), dir: path.slice(0, idx) };
}

export function FilesTab({
  repoId, worktreeForRepo, expandedDirs, onToggleDir, onOpenFile,
}: {
  repoId: string | null;
  worktreeForRepo: (id: string) => Worktree | undefined;
  expandedDirs: Set<string>;
  onToggleDir: (path: string) => void;
  onOpenFile: (path: string, repoId: string, lang: string) => void;
}) {
  const diff = useSession((s) => s.diff);
  const panelFocusNonce = useStore((s) => s.panelFocusNonce);
  const fileSearchFocusNonce = useStore((s) => s.fileSearchFocusNonce);
  const openTab = useSession((s) => s.openTab);
  const commitPreview = useSession((s) => s.commitPreview);
  const discardPreview = useSession((s) => s.discardPreview);
  const setActiveTab = useSession((s) => s.setActiveTab);
  const activePaneId = useSession((s) => s.activePaneId);
  const activePaneTabs = useSession((s) => s.panes.find((p) => p.id === s.activePaneId)?.tabs ?? []);
  const activeTabIdLive = useSession((s) => s.panes.find((p) => p.id === s.activePaneId)?.activeTabId ?? null);

  // Files already open as a real (non-preview) tab — previewing one of these must
  // NOT switch to it; only Enter (commit) should. Keyed like openTabReducer's file
  // tabs (`${repoId}::${path}`).
  const openTabKeys = useMemo(
    () => new Set(activePaneTabs.filter((t) => !t.preview && t.kind !== 'changes').map((t) => t.id)),
    [activePaneTabs],
  );

  const [tree, setTree] = useState<TreeNode[]>([]);
  const [files, setFiles] = useState<string[]>([]);
  const [loadingFiles, setLoadingFiles] = useState(false);
  const [query, setQuery] = useState('');
  const [selectedIdx, setSelectedIdx] = useState(0);

  const inputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<number | null>(null);
  // Live during a search session: where to put focus back on Esc.
  const sessionRef = useRef<{ paneId: string; prevTabId: string | null } | null>(null);

  const notify = useStore((s) => s.notify);
  const setLastError = useStore((s) => s.setLastError);
  const setGrepHighlight = useStore((s) => s.setGrepHighlight);

  // Search mode: fuzzy filename ('name') or content/grep ('text').
  const [mode, setMode] = useState<SearchMode>('name');
  const [grepFiles, setGrepFiles] = useState<GrepFile[]>([]);
  const [grepLoading, setGrepLoading] = useState(false);
  // Collapsed file groups, by path. Reset on a new query.
  const [grepCollapsed, setGrepCollapsed] = useState<Set<string>>(new Set());
  const rows = useMemo(() => grepRows(grepFiles, grepCollapsed), [grepFiles, grepCollapsed]);

  const wt = repoId ? worktreeForRepo(repoId) : undefined;
  const wtPath = wt?.path;

  const loadFiles = useCallback((isCancelled?: () => boolean) => {
    if (!wtPath) return;
    setLoadingFiles(true);
    invoke<string[]>('list_files', { worktreePath: wtPath })
      .then((f) => { if (isCancelled?.()) return; setFiles(f); setTree(buildTree(f)); })
      .catch(console.error)
      .finally(() => { if (!isCancelled?.()) setLoadingFiles(false); });
  }, [wtPath]);
  // Guard against a stale list_files response (from a previous worktree) landing
  // after a newer one and clobbering the tree.
  useEffect(() => {
    let cancelled = false;
    loadFiles(() => cancelled);
    return () => { cancelled = true; };
  }, [loadFiles]);

  // ── File-tree CRUD (context menu → create / rename / move / copy / delete) ──
  const [menu, setMenu] = useState<{ x: number; y: number; node: TreeNode | null } | null>(null);
  const [prompt, setPrompt] = useState<
    null | { title: string; initial: string; confirmLabel?: string; run: (name: string) => void }
  >(null);
  const [confirmDel, setConfirmDel] = useState<TreeNode | null>(null);
  const [clipboard, setClipboard] = useState<TreeClipboard | null>(null);

  // Every existing path (files + their ancestor dirs) — for collision-free copies.
  const allPaths = useMemo(() => {
    const s = new Set<string>(files);
    for (const p of files) {
      const parts = p.split('/');
      for (let i = 1; i < parts.length; i++) s.add(parts.slice(0, i).join('/'));
    }
    return s;
  }, [files]);

  const dirName = (p: string) => { const i = p.lastIndexOf('/'); return i === -1 ? '' : p.slice(0, i); };
  const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i === -1 ? p : p.slice(i + 1); };
  const joinPath = (dir: string, name: string) => (dir ? `${dir}/${name}` : name);
  const targetDir = (node: TreeNode | null) => (!node ? '' : node.isDir ? node.path : dirName(node.path));
  const uniqueDest = (dir: string, base: string) => {
    const dot = base.lastIndexOf('.');
    const stem = dot > 0 ? base.slice(0, dot) : base;
    const ext = dot > 0 ? base.slice(dot) : '';
    let name = base;
    let i = 1;
    while (allPaths.has(joinPath(dir, name))) { name = `${stem} copy${i > 1 ? ' ' + i : ''}${ext}`; i++; }
    return joinPath(dir, name);
  };

  const runFsOp = useCallback((cmd: string, args: Record<string, unknown>, okMsg: string, after?: () => void) => {
    invoke(cmd, args)
      .then(() => { loadFiles(); notify({ kind: 'success', source: 'files', title: okMsg }); after?.(); })
      .catch((e) => setLastError(String(e)));
  }, [loadFiles, notify, setLastError]);

  const onMenuAction = (a: MenuAction, node: TreeNode | null) => {
    if (!wtPath) return;
    const dir = targetDir(node);
    switch (a) {
      case 'newFile':
        setPrompt({ title: 'New file', initial: '', confirmLabel: 'Create', run: (name) => {
          const path = joinPath(dir, name);
          runFsOp('create_file', { worktreePath: wtPath, path }, `Created ${name}`, () => {
            if (dir && !expandedDirs.has(dir)) onToggleDir(dir);
            if (repoId) openTab({ repoId, filePath: path, view: 'edit' });
          });
        } });
        break;
      case 'newFolder':
        setPrompt({ title: 'New folder', initial: '', confirmLabel: 'Create', run: (name) => {
          const path = joinPath(dir, name);
          runFsOp('create_directory', { worktreePath: wtPath, path }, `Created ${name}/`, () => {
            if (!expandedDirs.has(path)) onToggleDir(path);
          });
        } });
        break;
      case 'rename':
        if (!node) break;
        setPrompt({ title: 'Rename', initial: baseName(node.path), confirmLabel: 'Rename', run: (name) => {
          runFsOp('rename_path', { worktreePath: wtPath, from: node.path, to: joinPath(dirName(node.path), name) }, `Renamed to ${name}`);
        } });
        break;
      case 'duplicate':
        if (!node) break;
        runFsOp('copy_path', { worktreePath: wtPath, from: node.path, to: uniqueDest(dirName(node.path), baseName(node.path)) }, `Duplicated ${baseName(node.path)}`);
        break;
      case 'copy': if (node) setClipboard({ path: node.path, mode: 'copy' }); break;
      case 'cut': if (node) setClipboard({ path: node.path, mode: 'cut' }); break;
      case 'paste': {
        if (!clipboard) break;
        const base = baseName(clipboard.path);
        if (clipboard.mode === 'copy') {
          runFsOp('copy_path', { worktreePath: wtPath, from: clipboard.path, to: uniqueDest(dir, base) }, `Copied ${base}`);
        } else {
          runFsOp('rename_path', { worktreePath: wtPath, from: clipboard.path, to: joinPath(dir, base) }, `Moved ${base}`, () => setClipboard(null));
        }
        break;
      }
      case 'copyRelPath':
        if (node) copyPath(node.path, 'Relative path copied');
        break;
      case 'copyAbsPath':
        if (node) copyPath(joinPath(wtPath, node.path), 'Absolute path copied');
        break;
      case 'delete': if (node) setConfirmDel(node); break;
    }
  };

  const copyPath = (text: string, done: string) => {
    invoke('copy_to_clipboard', { text })
      .then(() => notify({ kind: 'success', source: 'files', title: done, detail: text }))
      .catch((e) => setLastError(String(e)));
  };

  const doDelete = () => {
    if (!wtPath || !confirmDel) return;
    const node = confirmDel;
    setConfirmDel(null);
    runFsOp('delete_path', { worktreePath: wtPath, path: node.path }, `Deleted ${baseName(node.path)}`);
  };

  // ── Search results (fuzzy, active-repo only) ──────────────────────────────
  const results = useMemo(() => {
    const q = query.trim();
    if (!q) return [];
    const ql = q.toLowerCase();
    const scored = files
      .map((f) => ({ f, ranges: matchRanges(q, f), sub: f.toLowerCase().indexOf(ql) }))
      .filter((r) => r.ranges !== null);
    scored.sort((a, b) => {
      const aSub = a.sub !== -1, bSub = b.sub !== -1;
      if (aSub !== bSub) return aSub ? -1 : 1;          // contiguous substring first
      if (aSub && bSub && a.sub !== b.sub) return a.sub - b.sub; // earlier match first
      if (a.f.length !== b.f.length) return a.f.length - b.f.length; // shorter path first
      return a.f.localeCompare(b.f);
    });
    return scored.slice(0, MAX_RESULTS);
  }, [files, query]);
  const searching = query.trim().length > 0;
  // Rows currently shown, per mode — drives navigation + Enter.
  const activeCount = mode === 'text' ? rows.length : results.length;
  const clampedSel = Math.min(selectedIdx, Math.max(0, activeCount - 1));

  // ── Content search (grep) ─────────────────────────────────────────────────
  // Runs when in text mode and groups the matches per file. The editor's highlight
  // is NOT set here: it marks the row the cursor is on, so it follows the selection
  // (previewGrep / commitGrep) rather than the query.
  useEffect(() => {
    if (mode !== 'text') { setGrepFiles([]); return; }
    const q = query.trim();
    if (q.length < 2 || !wtPath) { setGrepFiles([]); return; }
    setGrepLoading(true);
    let cancelled = false;
    const t = window.setTimeout(() => {
      invoke<SearchMatch[]>('search_files', { query: q, worktreePath: wtPath, caseSensitive: false, maxResults: 300 })
        .then((matches) => {
          if (cancelled) return;
          // Insertion order is the ripgrep order, which is already sorted by path.
          const byFile = new Map<string, SearchMatch[]>();
          for (const m of matches) {
            const e = byFile.get(m.file);
            if (e) e.push(m);
            else byFile.set(m.file, [m]);
          }
          setGrepFiles([...byFile.entries()].map(([file, ms]) => ({ file, matches: ms })));
        })
        .catch(() => { if (!cancelled) setGrepFiles([]); })
        .finally(() => { if (!cancelled) setGrepLoading(false); });
    }, GREP_DEBOUNCE_MS);
    return () => { cancelled = true; clearTimeout(t); };
  }, [mode, query, wtPath, setGrepHighlight]);

  // ── Transient preview ─────────────────────────────────────────────────────
  const previewPath = useCallback((path: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    // Don't preview a file that's already open as a real tab — that would yank
    // the view to it. It stays put; Enter switches to it (see commitPath).
    if (repoId && openTabKeys.has(`${repoId}::${path}`)) return;
    debounceRef.current = window.setTimeout(() => {
      debounceRef.current = null;
      if (repoId) openTab({ repoId, filePath: path, view: 'edit', preview: true });
    }, PREVIEW_DEBOUNCE_MS);
  }, [repoId, openTab, openTabKeys]);

  // Preview a grep hit at ITS line — a file header previews its first match, so
  // walking the list lands on the matching word either way, and that one match is
  // what the editor marks.
  const previewGrep = useCallback((file: string, line: number) => {
    if (!repoId) return;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => {
      debounceRef.current = null;
      setGrepHighlight({ query: query.trim(), line });
      openTab({ repoId, filePath: file, view: 'edit', cursorLine: line, preview: true });
    }, PREVIEW_DEBOUNCE_MS);
  }, [repoId, openTab, query, setGrepHighlight]);

  /** The line a row should open at. */
  const rowLine = (r: GrepRow) => (r.kind === 'match' ? r.match.line : r.file.matches[0]?.line ?? 0);

  // New query → cursor to top.
  useEffect(() => { setSelectedIdx(0); setGrepCollapsed(new Set()); }, [query]);
  // Preview whatever is highlighted (debounced), per mode.
  useEffect(() => {
    if (!searching) return;
    if (mode === 'text') {
      const r = rows[Math.min(selectedIdx, rows.length - 1)];
      if (r) previewGrep(r.file.file, rowLine(r));
    } else if (results.length) {
      previewPath(results[Math.min(selectedIdx, results.length - 1)].f);
    }
   
  }, [searching, mode, selectedIdx, results, rows, previewPath, previewGrep]);
  // Keep the cursor row visible.
  useEffect(() => {
    resultsRef.current?.querySelector('.nav-selected')?.scrollIntoView({ block: 'nearest' });
  }, [clampedSel, results, rows]);

  const endSearch = useCallback(() => {
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    sessionRef.current = null;
    setQuery('');
    setSelectedIdx(0);
  }, []);

  const commitPath = useCallback((path: string) => {
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    if (!repoId) return;
    // Already-open file: drop any lingering preview tab, then re-open it as a
    // normal tab (activates + focuses it — same feel as committing a preview).
    if (openTabKeys.has(`${repoId}::${path}`)) {
      discardPreview(activePaneId);
      openTab({ repoId, filePath: path, view: 'edit' });
    } else {
      openTab({ repoId, filePath: path, view: 'edit', preview: true }); // ensure the right file is the preview
      commitPreview(activePaneId);
    }
    endSearch();
  }, [repoId, openTab, commitPreview, discardPreview, openTabKeys, activePaneId, endSearch]);

  // Commit a grep result: open it as a real tab at the match line. The highlight is
  // kept — and re-set, because `endSearch` clears the query this reads — so the match
  // you chose is still marked in the tab you land in.
  const commitGrep = useCallback((file: string, line: number) => {
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    if (!repoId) return;
    discardPreview(activePaneId);
    setGrepHighlight({ query: query.trim(), line });
    openTab({ repoId, filePath: file, view: 'edit', cursorLine: line });
    endSearch();
  }, [repoId, openTab, discardPreview, activePaneId, endSearch, query, setGrepHighlight]);

  const commitSelected = useCallback(() => {
    if (mode === 'text') {
      const row = rows[clampedSel];
      if (row) commitGrep(row.file.file, rowLine(row));
    } else { const r = results[clampedSel]; if (r) commitPath(r.f); }
   
  }, [mode, rows, results, clampedSel, commitGrep, commitPath]);

  const cancelSearch = useCallback(() => {
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    const sess = sessionRef.current;
    discardPreview(activePaneId);
    if (sess?.prevTabId) setActiveTab(activePaneId, sess.prevTabId);
    setGrepHighlight(null);
    endSearch();
  }, [discardPreview, setActiveTab, activePaneId, endSearch, setGrepHighlight]);

  const onQueryChange = (v: string) => {
    if (!v.trim()) { if (sessionRef.current) cancelSearch(); else setQuery(''); return; }
    if (!sessionRef.current) sessionRef.current = { paneId: activePaneId, prevTabId: activeTabIdLive };
    setQuery(v);
  };

  const onInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    const ctrl = e.ctrlKey || e.metaKey;
    if ((ctrl && e.key === 'j') || e.key === 'ArrowDown') {
      e.preventDefault(); setSelectedIdx((i) => Math.min(i + 1, activeCount - 1));
    } else if ((ctrl && e.key === 'k') || e.key === 'ArrowUp') {
      e.preventDefault(); setSelectedIdx((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault(); commitSelected();
    } else if (e.key === 'Escape') {
      e.preventDefault(); cancelSearch();
    }
  };

  const switchMode = (m: SearchMode) => {
    if (m === mode) return;
    setMode(m);
    setSelectedIdx(0);
    if (m === 'name') setGrepHighlight(null);
    inputRef.current?.focus();
  };

  // Alt+F / Ctrl+Shift+F focus the search input (Ctrl+Shift+F selects text mode).
  const fileSearchMode = useStore((s) => s.fileSearchMode);
  useEffect(() => {
    if (!fileSearchFocusNonce) return;
    setMode(fileSearchMode);
    inputRef.current?.focus();
    inputRef.current?.select();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fileSearchFocusNonce]);

  // Live mirrors so the repo-switch + unmount cleanups read fresh values.
  const activePaneIdRef = useRef(activePaneId);
  activePaneIdRef.current = activePaneId;
  const discardRef = useRef({ discardPreview, setActiveTab });
  discardRef.current = { discardPreview, setActiveTab };

  // Repo switch → abandon any in-flight search/preview.
  useEffect(() => {
    if (sessionRef.current) discardRef.current.discardPreview(activePaneIdRef.current);
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    sessionRef.current = null;
    setQuery('');
    setSelectedIdx(0);
    useStore.getState().setGrepHighlight(null);
  }, [repoId]);

  // Unmount (panel/session switch) → drop a lingering preview + restore + clear
  // the editor match highlight.
  useEffect(() => () => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    useStore.getState().setGrepHighlight(null);
    if (sessionRef.current) {
      discardRef.current.discardPreview(activePaneIdRef.current);
      if (sessionRef.current.prevTabId) discardRef.current.setActiveTab(activePaneIdRef.current, sessionRef.current.prevTabId);
    }
  }, []);

  // ── Tree keyboard nav (only used when NOT searching) ──────────────────────
  const visible = useMemo(() => flattenVisible(tree, expandedDirs), [tree, expandedDirs]);
  const indexByPath = useMemo(() => {
    const m = new Map<string, number>();
    visible.forEach((v, i) => m.set(v.node.path, i));
    return m;
  }, [visible]);

  const onEnter = useCallback((i: number) => {
    const v = visible[i];
    if (!v || !repoId) return;
    if (v.node.isDir) onToggleDir(v.node.path);
    else onOpenFile(v.node.path, repoId, guessLang(v.node.path));
  }, [visible, repoId, onToggleDir, onOpenFile]);
  const onRight = useCallback((i: number) => {
    const v = visible[i];
    if (!v || !v.node.isDir) return;
    if (!expandedDirs.has(v.node.path)) { onToggleDir(v.node.path); return; }
    return i + 1;
  }, [visible, expandedDirs, onToggleDir]);
  const onLeft = useCallback((i: number) => {
    const v = visible[i];
    if (!v) return;
    if (v.node.isDir && expandedDirs.has(v.node.path)) { onToggleDir(v.node.path); return; }
    const parent = v.node.path.split('/').slice(0, -1).join('/');
    if (!parent) return;
    return indexByPath.get(parent);
  }, [visible, expandedDirs, indexByPath, onToggleDir]);

  const nav = useListNav({ count: visible.length, onEnter, onLeft, onRight, focusNonce: panelFocusNonce });
  const selectedPath = visible[nav.index]?.node.path ?? null;

  if (!repoId) return <div className="sidebar-empty">Select a repo above</div>;
  if (!wt) return <div className="sidebar-empty">No active worktree</div>;

  const changedPaths = new Set(
    diff?.repos.find((r) => r.repo_id === repoId)?.files.map((f) => f.path) ?? []
  );

  return (
    <div className="files-panel">
      <div className="file-search">
        <Search className="file-search-icon" size={14} strokeWidth={2} />
        <input
          ref={inputRef}
          data-file-search="1"
          className="file-search-input"
          placeholder={mode === 'text' ? 'Search in files…' : 'Find file…'}
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={onInputKeyDown}
          spellCheck={false}
        />
        <div className="file-search-modes" role="group" aria-label="Search mode">
          <button
            className={`file-search-mode ${mode === 'name' ? 'active' : ''}`}
            title="Find file by name"
            onMouseDown={(e) => { e.preventDefault(); switchMode('name'); }}
          >
            <Search size={13} strokeWidth={2} />
          </button>
          <button
            className={`file-search-mode ${mode === 'text' ? 'active' : ''}`}
            title="Search file contents"
            onMouseDown={(e) => { e.preventDefault(); switchMode('text'); }}
          >
            <TextSearch size={13} strokeWidth={2} />
          </button>
        </div>
        {searching && (
          <button className="file-search-clear" title="Clear (Esc)" onMouseDown={(e) => { e.preventDefault(); cancelSearch(); }}>
            <X size={13} strokeWidth={2} />
          </button>
        )}
      </div>

      {loadingFiles ? (
        <div className="sidebar-empty">Loading…</div>
      ) : searching && mode === 'text' ? (
        <div className="files-list file-search-results" ref={resultsRef}>
          {query.trim().length < 2 ? (
            <div className="sidebar-empty">Type at least 2 characters</div>
          ) : grepLoading && grepFiles.length === 0 ? (
            <div className="sidebar-empty">Searching…</div>
          ) : grepFiles.length === 0 ? (
            <div className="sidebar-empty">No matches</div>
          ) : (
            rows.map((row, i) => {
              const selected = i === clampedSel ? 'nav-selected' : '';
              if (row.kind === 'file') {
                const { name, dir } = splitPath(row.file.file);
                const collapsed = grepCollapsed.has(row.file.file);
                const n = row.file.matches.length;
                return (
                  <button
                    key={row.file.file}
                    className={`file-item grep-file ${selected}`}
                    title={`${row.file.file} — ${n} match${n === 1 ? '' : 'es'}`}
                    tabIndex={-1}
                    onMouseEnter={() => setSelectedIdx(i)}
                    onClick={(e) => {
                      // The chevron collapses; the row itself opens the first match,
                      // which is what it did before the grouping.
                      if ((e.target as HTMLElement).closest('.grep-collapse')) return;
                      commitGrep(row.file.file, rowLine(row));
                    }}
                  >
                    <span
                      className="grep-collapse"
                      role="presentation"
                      onClick={() => setGrepCollapsed((prev) => {
                        const next = new Set(prev);
                        if (next.has(row.file.file)) next.delete(row.file.file);
                        else next.add(row.file.file);
                        return next;
                      })}
                    >
                      {collapsed ? <ChevronRight size={12} strokeWidth={2} /> : <ChevronDown size={12} strokeWidth={2} />}
                    </span>
                    <span className="file-icon"><FileTypeIcon name={name} /></span>
                    <span className="file-name">{name}</span>
                    {dir && <span className="file-search-dir">{dir}</span>}
                    <span className="file-search-count">{n}</span>
                  </button>
                );
              }
              return (
                <button
                  key={`${row.file.file}:${row.match.line}`}
                  className={`file-item grep-match ${selected}`}
                  title={`${row.file.file}:${row.match.line}`}
                  tabIndex={-1}
                  onMouseEnter={() => setSelectedIdx(i)}
                  onClick={() => commitGrep(row.file.file, row.match.line)}
                >
                  <span className="grep-match-line">{row.match.line}</span>
                  {/* The matched text is highlighted, so the eye lands on the hit
                      rather than the middle of a long line. */}
                  <span className="grep-match-text">
                    <Highlighted text={row.match.content.trim()} ranges={matchRanges(query, row.match.content.trim())} />
                  </span>
                </button>
              );
            })
          )}
        </div>
      ) : searching ? (
        <div className="files-list file-search-results" ref={resultsRef}>
          {results.length === 0 ? (
            <div className="sidebar-empty">No matching files</div>
          ) : (
            results.map((r, i) => {
              const { name, dir } = splitPath(r.f);
              return (
                <button
                  key={r.f}
                  className={`file-item ${i === clampedSel ? 'nav-selected' : ''}`}
                  title={r.f}
                  tabIndex={-1}
                  onMouseEnter={() => setSelectedIdx(i)}
                  onClick={() => commitPath(r.f)}
                >
                  <span className="file-icon"><FileTypeIcon name={name} /></span>
                  <span className="file-name"><Highlighted text={name} ranges={matchRanges(query, name)} /></span>
                  {dir && <span className="file-search-dir">{dir}</span>}
                </button>
              );
            })
          )}
        </div>
      ) : tree.length === 0 ? (
        <div className="sidebar-empty">No files</div>
      ) : (
        <div
          className="files-list nav-list"
          tabIndex={0}
          ref={nav.containerRef}
          onKeyDown={nav.onKeyDown}
          onContextMenu={(e) => { e.preventDefault(); setMenu({ x: e.clientX, y: e.clientY, node: null }); }}
        >
          <FileTreeNodes
            nodes={tree}
            depth={0}
            modifiedPaths={changedPaths}
            repoId={repoId}
            expandedDirs={expandedDirs}
            onToggleDir={onToggleDir}
            onOpenFile={onOpenFile}
            // Tree files have no diff view, so double-click reuses the open handler.
            onOpenFileAlt={onOpenFile}
            selectedPath={selectedPath}
            onSelect={(path) => nav.setIndex(indexByPath.get(path) ?? 0)}
            onContextMenu={(node, e) => setMenu({ x: e.clientX, y: e.clientY, node })}
          />
        </div>
      )}

      {menu && (
        <TreeContextMenu
          x={menu.x} y={menu.y} node={menu.node} hasClipboard={!!clipboard}
          onAction={onMenuAction} onClose={() => setMenu(null)}
        />
      )}
      {prompt && (
        <TreePrompt
          title={prompt.title} initialValue={prompt.initial} confirmLabel={prompt.confirmLabel}
          onSubmit={(v) => { prompt.run(v); setPrompt(null); }} onCancel={() => setPrompt(null)}
        />
      )}
      {confirmDel && (
        <TreeConfirmDelete node={confirmDel} onConfirm={doDelete} onCancel={() => setConfirmDel(null)} />
      )}
    </div>
  );
}
