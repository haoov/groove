import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { useListNav } from '../shared/lib/hooks/useListNav';
import { guessLang } from '../shared/lib/pure/lang';
import { buildTree, flattenVisible, type TreeNode } from './tree';

/** The worktree's file list, its tree, and the keyboard cursor walking it. */
export function useFileTree({
  repoId, wtPath, expandedDirs, onToggleDir, onOpenFile,
}: {
  repoId: string | null;
  wtPath?: string;
  expandedDirs: Set<string>;
  onToggleDir: (path: string) => void;
  onOpenFile: (path: string, repoId: string, lang: string) => void;
}) {
  const panelFocusNonce = useStore((s) => s.panelFocusNonce);
  const [tree, setTree] = useState<TreeNode[]>([]);
  const [files, setFiles] = useState<string[]>([]);
  const [loadingFiles, setLoadingFiles] = useState(false);

  const loadFiles = useCallback((isCancelled?: () => boolean) => {
    if (!wtPath) return;
    setLoadingFiles(true);
    invoke<string[]>('list_files', { worktreePath: wtPath })
      .then((f) => { if (isCancelled?.()) return; setFiles(f); setTree(buildTree(f)); })
      .catch(console.error)
      .finally(() => { if (!isCancelled?.()) setLoadingFiles(false); });
  }, [wtPath]);
  // A stale list_files response from a previous worktree is dropped.
  useEffect(() => {
    let cancelled = false;
    loadFiles(() => cancelled);
    return () => { cancelled = true; };
  }, [loadFiles]);

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
  const selectRow = (path: string) => nav.setIndex(indexByPath.get(path) ?? 0);

  return { tree, files, loadingFiles, loadFiles, nav, selectedPath, selectRow };
}
