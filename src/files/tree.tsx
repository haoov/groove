import {
  FileCode, FileJson, FileText, File, Database, Terminal,
  Folder, FolderOpen,
} from 'lucide-react';
import { fileIconColor } from '../shared/lib/icons';
import { guessLang } from '../shared/lib/lang';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  children: TreeNode[];
}

export type { DiffStat } from '../shared/ui/StatBadge';
import { StatBadge, type DiffStat } from '../shared/ui/StatBadge';

// ── Tree builders ─────────────────────────────────────────────────────────────

export function buildTree(paths: string[]): TreeNode[] {
  const root: TreeNode = { name: '', path: '', isDir: true, children: [] };
  for (const filePath of paths) {
    const parts = filePath.split('/');
    let cur = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const nodePath = parts.slice(0, i + 1).join('/');
      const isLast = i === parts.length - 1;
      let child = cur.children.find((c) => c.name === part);
      if (!child) {
        child = { name: part, path: nodePath, isDir: !isLast, children: [] };
        cur.children.push(child);
      }
      if (!isLast) cur = child;
    }
  }
  const sort = (nodes: TreeNode[]) => {
    nodes.sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    nodes.forEach((n) => n.isDir && sort(n.children));
  };
  sort(root.children);
  return root.children;
}

export interface FlatNode { node: TreeNode; depth: number }

/** Pre-order flatten of the rows currently visible (respecting expanded dirs). */
export function flattenVisible(
  nodes: TreeNode[], expanded: Set<string>, depth = 0, acc: FlatNode[] = []
): FlatNode[] {
  for (const n of nodes) {
    acc.push({ node: n, depth });
    if (n.isDir && expanded.has(n.path)) flattenVisible(n.children, expanded, depth + 1, acc);
  }
  return acc;
}

export function collectDirPaths(nodes: TreeNode[], acc: string[] = []): string[] {
  for (const n of nodes) {
    if (n.isDir) {
      acc.push(n.path);
      collectDirPaths(n.children, acc);
    }
  }
  return acc;
}

// ── Shared sub-components ─────────────────────────────────────────────────────

export function FileTypeIcon({ name }: { name: string }) {
  const ext = (name.split('/').pop() ?? name).split('.').pop()?.toLowerCase() ?? '';
  const color = fileIconColor(name);
  const props = { size: 16, strokeWidth: 1.5, style: { color, flexShrink: 0 } } as const;
  const codeExts = new Set(['ts','tsx','js','jsx','rs','py','go','c','cpp','cs','java','kt','swift','rb','html','css','scss','vue','svelte','php','dart']);
  if (codeExts.has(ext)) return <FileCode {...props} />;
  if (ext === 'json' || ext === 'jsonc') return <FileJson {...props} />;
  if (ext === 'md' || ext === 'mdx') return <FileText {...props} />;
  if (ext === 'sql') return <Database {...props} />;
  if (['sh', 'bash', 'zsh'].includes(ext)) return <Terminal {...props} />;
  return <File {...props} />;
}

export function FileTreeNodes({
  nodes, depth, modifiedPaths, repoId, expandedDirs, onToggleDir, onOpenFile, onOpenFileAlt, statsByPath,
  selectedPath, onSelect, onContextMenu,
}: {
  nodes: TreeNode[];
  depth: number;
  modifiedPaths: Set<string>;
  repoId: string;
  expandedDirs: Set<string>;
  onToggleDir: (path: string) => void;
  onOpenFile: (path: string, repoId: string, lang: string) => void;
  onOpenFileAlt: (path: string, repoId: string, lang: string) => void;
  statsByPath?: Record<string, DiffStat>;
  /** Path of the keyboard-cursor row (gets `nav-selected`). */
  selectedPath?: string | null;
  /** Notify the owner that a row was clicked, so it can sync its cursor. */
  onSelect?: (path: string) => void;
  /** Right-click a node → open the file-ops context menu. */
  onContextMenu?: (node: TreeNode, e: React.MouseEvent) => void;
}) {
  const indent = depth * 12;

  return (
    <>
      {nodes.map((node) =>
        node.isDir ? (
          <div key={node.path}>
            <button
              className={`file-tree-dir ${node.path === selectedPath ? 'nav-selected' : ''}`}
              style={{ paddingLeft: `${8 + indent}px` }}
              tabIndex={-1}
              onClick={() => { onSelect?.(node.path); onToggleDir(node.path); }}
              onContextMenu={onContextMenu ? (e) => { e.preventDefault(); e.stopPropagation(); onContextMenu(node, e); } : undefined}
            >
              <span className="file-tree-arrow">
                {expandedDirs.has(node.path)
                  ? <FolderOpen size={16} strokeWidth={1.5}/>
                  : <Folder size={16} strokeWidth={1.5}/>
                }
              </span>
              <span className="file-tree-dirname">{node.name}</span>
            </button>
            {expandedDirs.has(node.path) && (
              <FileTreeNodes
                nodes={node.children}
                depth={depth + 1}
                modifiedPaths={modifiedPaths}
                repoId={repoId}
                expandedDirs={expandedDirs}
                onToggleDir={onToggleDir}
                onOpenFile={onOpenFile}
                onOpenFileAlt={onOpenFileAlt}
                statsByPath={statsByPath}
                selectedPath={selectedPath}
                onSelect={onSelect}
                onContextMenu={onContextMenu}
              />
            )}
          </div>
        ) : (
          <button
            key={node.path}
            className={`file-item ${modifiedPaths.has(node.path) ? 'file-item-modified' : ''} ${node.path === selectedPath ? 'nav-selected' : ''}`}
            style={{ paddingLeft: `${8 + indent}px` }}
            title={node.path}
            tabIndex={-1}
            onClick={() => { onSelect?.(node.path); onOpenFile(node.path, repoId, guessLang(node.path)); }}
            onDoubleClick={() => onOpenFileAlt(node.path, repoId, guessLang(node.path))}
            onContextMenu={onContextMenu ? (e) => { e.preventDefault(); e.stopPropagation(); onContextMenu(node, e); } : undefined}
          >
            <span className="file-icon">
              <FileTypeIcon name={node.name}/>
            </span>
            {!statsByPath && modifiedPaths.has(node.path) && <span className="file-dot-modified" />}
            <span className="file-name">{node.name}</span>
            {statsByPath?.[node.path] && <StatBadge stat={statsByPath[node.path]} />}
          </button>
        )
      )}
    </>
  );
}
