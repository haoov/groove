import { useCallback, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useSession, useStore } from '../shared/store';
import type { TreeNode } from './tree';
import type { MenuAction, TreeClipboard } from './FileTreeMenu';

const dirName = (p: string) => { const i = p.lastIndexOf('/'); return i === -1 ? '' : p.slice(0, i); };
const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i === -1 ? p : p.slice(i + 1); };
const joinPath = (dir: string, name: string) => (dir ? `${dir}/${name}` : name);
const targetDir = (node: TreeNode | null) => (!node ? '' : node.isDir ? node.path : dirName(node.path));

/** File-tree CRUD (context menu → create / rename / move / copy / delete). */
export function useTreeOps({
  repoId, wtPath, files, expandedDirs, onToggleDir, reload,
}: {
  repoId: string | null;
  wtPath?: string;
  files: string[];
  expandedDirs: Set<string>;
  onToggleDir: (path: string) => void;
  reload: () => void;
}) {
  const openTab = useSession((s) => s.openTab);
  const notify = useStore((s) => s.notify);
  const setLastError = useStore((s) => s.setLastError);

  const [menu, setMenu] = useState<{ x: number; y: number; node: TreeNode | null } | null>(null);
  const [prompt, setPrompt] = useState<
    null | { title: string; initial: string; confirmLabel?: string; run: (name: string) => void }
  >(null);
  const [confirmDel, setConfirmDel] = useState<TreeNode | null>(null);
  const [clipboard, setClipboard] = useState<TreeClipboard | null>(null);

  // Every existing path: files and their ancestor dirs.
  const allPaths = useMemo(() => {
    const s = new Set<string>(files);
    for (const p of files) {
      const parts = p.split('/');
      for (let i = 1; i < parts.length; i++) s.add(parts.slice(0, i).join('/'));
    }
    return s;
  }, [files]);

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
      .then(() => { reload(); notify({ kind: 'success', source: 'files', title: okMsg }); after?.(); })
      .catch((e) => setLastError(e));
  }, [reload, notify, setLastError]);

  const copyPath = (text: string, done: string) => {
    invoke('copy_to_clipboard', { text })
      .then(() => notify({ kind: 'success', source: 'files', title: done, detail: text }))
      .catch((e) => setLastError(e));
  };

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
        if (node) copyPath(node.path, 'Relative path sent to the clipboard');
        break;
      case 'copyAbsPath':
        if (node) copyPath(joinPath(wtPath, node.path), 'Absolute path sent to the clipboard');
        break;
      case 'delete': if (node) setConfirmDel(node); break;
    }
  };

  const doDelete = () => {
    if (!wtPath || !confirmDel) return;
    const node = confirmDel;
    setConfirmDel(null);
    runFsOp('delete_path', { worktreePath: wtPath, path: node.path }, `Deleted ${baseName(node.path)}`);
  };

  return { menu, setMenu, prompt, setPrompt, confirmDel, setConfirmDel, clipboard, onMenuAction, doDelete };
}
