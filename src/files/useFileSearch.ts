import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useSession, useStore } from '../shared/store';
import { matchRanges } from '../shared/lib/match';
import type { SearchMatch } from '../shared/ipc/ipc';

const PREVIEW_DEBOUNCE_MS = 80;
const GREP_DEBOUNCE_MS = 160;
const MAX_RESULTS = 50;

export type SearchMode = 'name' | 'text';
/** One file's matches, rendered as a header with its hits under it. */
export interface GrepFile { file: string; matches: SearchMatch[] }

/** The flattened list the keyboard walks: a file header, then its matches. */
export type GrepRow =
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

/** The line a row should open at. */
export const rowLine = (r: GrepRow) => (r.kind === 'match' ? r.match.line : r.file.matches[0]?.line ?? 0);

export type FileSearch = ReturnType<typeof useFileSearch>;

/** Fuzzy name search and grep over one worktree, previewing the highlighted row. */
export function useFileSearch({
  repoId, wtPath, files,
}: {
  repoId: string | null;
  wtPath?: string;
  files: string[];
}) {
  const openTab = useSession((s) => s.openTab);
  const commitPreview = useSession((s) => s.commitPreview);
  const discardPreview = useSession((s) => s.discardPreview);
  const setActiveTab = useSession((s) => s.setActiveTab);
  const activePaneId = useSession((s) => s.activePaneId);
  const activePaneTabs = useSession((s) => s.panes.find((p) => p.id === s.activePaneId)?.tabs ?? []);
  const activeTabIdLive = useSession((s) => s.panes.find((p) => p.id === s.activePaneId)?.activeTabId ?? null);
  const fileSearchFocusNonce = useStore((s) => s.fileSearchFocusNonce);
  const setGrepHighlight = useStore((s) => s.setGrepHighlight);

  // Real (non-preview) open tabs, keyed as openTabReducer keys file tabs: `${repoId}::${path}`.
  const openTabKeys = useMemo(
    () => new Set(activePaneTabs.filter((t) => !t.preview && t.kind !== 'changes').map((t) => t.id)),
    [activePaneTabs],
  );

  const [query, setQuery] = useState('');
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [mode, setMode] = useState<SearchMode>('name');
  const [grepFiles, setGrepFiles] = useState<GrepFile[]>([]);
  const [grepLoading, setGrepLoading] = useState(false);
  // Collapsed file groups, by path.
  const [grepCollapsed, setGrepCollapsed] = useState<Set<string>>(new Set());
  const rows = useMemo(() => grepRows(grepFiles, grepCollapsed), [grepFiles, grepCollapsed]);

  const inputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<number | null>(null);
  // Set during a search session: where Esc returns focus.
  const sessionRef = useRef<{ paneId: string; prevTabId: string | null } | null>(null);

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
  const activeCount = mode === 'text' ? rows.length : results.length;
  const clampedSel = Math.min(selectedIdx, Math.max(0, activeCount - 1));

  // ── Content search (grep) ─────────────────────────────────────────────────
  // The editor highlight is set by previewGrep / commitGrep, not here.
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
          // Insertion order is ripgrep's path order.
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
    // No preview for a file open as a real tab; Enter switches to it (commitPath).
    if (repoId && openTabKeys.has(`${repoId}::${path}`)) return;
    debounceRef.current = window.setTimeout(() => {
      debounceRef.current = null;
      if (repoId) openTab({ repoId, filePath: path, view: 'edit', preview: true });
    }, PREVIEW_DEBOUNCE_MS);
  }, [repoId, openTab, openTabKeys]);

  // Preview a grep hit at its line; a file header previews its first match.
  const previewGrep = useCallback((file: string, line: number) => {
    if (!repoId) return;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => {
      debounceRef.current = null;
      setGrepHighlight({ query: query.trim(), line });
      openTab({ repoId, filePath: file, view: 'edit', cursorLine: line, preview: true });
    }, PREVIEW_DEBOUNCE_MS);
  }, [repoId, openTab, query, setGrepHighlight]);

  // New query: cursor to top. Set during render; an effect paints one frame with the stale cursor.
  const [cursorQuery, setCursorQuery] = useState(query);
  if (cursorQuery !== query) {
    setCursorQuery(query);
    setSelectedIdx(0);
    setGrepCollapsed(new Set());
  }
  // Preview the highlighted row, debounced.
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
    // An open file: drop any preview tab, then re-open it as a normal tab.
    if (openTabKeys.has(`${repoId}::${path}`)) {
      discardPreview(activePaneId);
      openTab({ repoId, filePath: path, view: 'edit' });
    } else {
      openTab({ repoId, filePath: path, view: 'edit', preview: true }); // make this file the preview
      commitPreview(activePaneId);
    }
    endSearch();
  }, [repoId, openTab, commitPreview, discardPreview, openTabKeys, activePaneId, endSearch]);

  // Open a grep result as a real tab at its line. Re-set the highlight before `endSearch` clears the query.
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
  // deps omit `fileSearchMode`: the mode applies only on a focus request.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fileSearchFocusNonce]);

  // Refs for the repo-switch and unmount cleanups.
  const activePaneIdRef = useRef(activePaneId);
  activePaneIdRef.current = activePaneId;
  const discardRef = useRef({ discardPreview, setActiveTab });
  discardRef.current = { discardPreview, setActiveTab };

  // Repo switch: abandon any search and preview.
  useEffect(() => {
    if (sessionRef.current) discardRef.current.discardPreview(activePaneIdRef.current);
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    sessionRef.current = null;
    setQuery('');
    setSelectedIdx(0);
    useStore.getState().setGrepHighlight(null);
  }, [repoId]);

  // Unmount: drop a lingering preview, restore the tab, clear the highlight.
  useEffect(() => () => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    useStore.getState().setGrepHighlight(null);
    if (sessionRef.current) {
      discardRef.current.discardPreview(activePaneIdRef.current);
      if (sessionRef.current.prevTabId) discardRef.current.setActiveTab(activePaneIdRef.current, sessionRef.current.prevTabId);
    }
  }, []);

  return {
    query, onQueryChange, onInputKeyDown, searching, cancelSearch, inputRef, resultsRef,
    mode, switchMode, results, rows, clampedSel, setSelectedIdx,
    grepFiles, grepLoading, grepCollapsed, setGrepCollapsed, commitPath, commitGrep,
  };
}
