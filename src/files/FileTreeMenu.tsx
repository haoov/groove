import { useEffect, useRef, useState } from 'react';
import {
  FilePlus, FolderPlus, Pencil, Copy, Scissors, ClipboardPaste, Files, Trash2, Link2,
} from 'lucide-react';
import type { TreeNode } from './tree';
import { ContextMenu } from '../shared/ui/ContextMenu';

export type MenuAction =
  | 'newFile' | 'newFolder' | 'rename' | 'duplicate' | 'copy' | 'cut' | 'paste' | 'delete'
  | 'copyRelPath' | 'copyAbsPath';

export interface TreeClipboard { path: string; mode: 'copy' | 'cut' }

interface MenuItem { action: MenuAction; label: string; Icon: typeof FilePlus; danger?: boolean }

/** Right-click menu for a tree node (or the empty/root area when node is null). */
export function TreeContextMenu({
  x, y, node, hasClipboard, onAction, onClose,
}: {
  x: number;
  y: number;
  node: TreeNode | null;
  hasClipboard: boolean;
  onAction: (a: MenuAction, node: TreeNode | null) => void;
  onClose: () => void;
}) {
  const isFile = !!node && !node.isDir;
  const items: (MenuItem | 'sep')[] = [
    { action: 'newFile', label: 'New file', Icon: FilePlus },
    { action: 'newFolder', label: 'New folder', Icon: FolderPlus },
  ];
  if (node) {
    items.push('sep');
    items.push({ action: 'rename', label: 'Rename', Icon: Pencil });
    if (isFile) items.push({ action: 'duplicate', label: 'Duplicate', Icon: Files });
    items.push({ action: 'copy', label: 'Copy', Icon: Copy });
    items.push({ action: 'cut', label: 'Cut', Icon: Scissors });
    items.push('sep');
    items.push({ action: 'copyRelPath', label: 'Copy relative path', Icon: Link2 });
    items.push({ action: 'copyAbsPath', label: 'Copy absolute path', Icon: Link2 });
  }
  if (hasClipboard) items.push({ action: 'paste', label: 'Paste', Icon: ClipboardPaste });
  if (node) {
    items.push('sep');
    items.push({ action: 'delete', label: 'Delete', Icon: Trash2, danger: true });
  }

  return (
    <ContextMenu x={x} y={y} onClose={onClose} className="ctx-menu">
      {items.map((it, i) =>
        it === 'sep' ? (
          <div key={`sep-${i}`} className="ctx-menu-sep" />
        ) : (
          <button
            key={it.action}
            className={`ctx-menu-item ${it.danger ? 'ctx-menu-item--danger' : ''}`}
            onClick={() => { onAction(it.action, node); onClose(); }}
          >
            <it.Icon size={14} strokeWidth={1.75} />
            <span>{it.label}</span>
          </button>
        )
      )}
    </ContextMenu>
  );
}

/** Centered single-line prompt (create/rename). Enter confirms, Esc cancels. */
export function TreePrompt({
  title, initialValue, placeholder, confirmLabel = 'OK', onSubmit, onCancel,
}: {
  title: string;
  initialValue: string;
  placeholder?: string;
  confirmLabel?: string;
  onSubmit: (value: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(initialValue);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    // Select the base name (before the extension) for a rename.
    const dot = initialValue.lastIndexOf('.');
    inputRef.current?.setSelectionRange(0, dot > 0 ? dot : initialValue.length);
  }, [initialValue]);

  const submit = () => { const v = value.trim(); if (v) onSubmit(v); };

  return (
    <div className="tree-prompt-overlay" onMouseDown={onCancel}>
      <div className="tree-prompt" onMouseDown={(e) => e.stopPropagation()}>
        <div className="tree-prompt-title">{title}</div>
        <input
          ref={inputRef}
          className="tree-prompt-input"
          value={value}
          placeholder={placeholder}
          spellCheck={false}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') { e.preventDefault(); submit(); }
            else if (e.key === 'Escape') { e.preventDefault(); onCancel(); }
          }}
        />
        <div className="tree-prompt-actions">
          <button className="tree-prompt-cancel" onClick={onCancel}>Cancel</button>
          <button className="tree-prompt-ok" onClick={submit}>{confirmLabel}</button>
        </div>
      </div>
    </div>
  );
}

/** Centered destructive confirm for deleting a node. */
export function TreeConfirmDelete({
  node, onConfirm, onCancel,
}: {
  node: TreeNode;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const popoverRef = useRef<HTMLDivElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  // Focus the Delete button on mount so keyboard confirm is scoped here.
  useEffect(() => { confirmRef.current?.focus(); }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { onCancel(); return; }
      // Enter must confirm ONLY when focus is inside this popover — otherwise
      // Enter in the editor would trigger a permanent delete.
      if (e.key === 'Enter' && popoverRef.current?.contains(document.activeElement)) onConfirm();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [onConfirm, onCancel]);

  return (
    <div className="tree-prompt-overlay" onMouseDown={onCancel}>
      <div className="tree-prompt" ref={popoverRef} onMouseDown={(e) => e.stopPropagation()}>
        <div className="tree-prompt-title">Delete {node.isDir ? 'folder' : 'file'}?</div>
        <div className="tree-prompt-body">
          <code>{node.path}</code>{node.isDir ? ' and everything inside it' : ''} will be permanently deleted.
        </div>
        <div className="tree-prompt-actions">
          <button className="tree-prompt-cancel" onClick={onCancel}>Cancel</button>
          <button ref={confirmRef} className="tree-prompt-ok tree-prompt-ok--danger" onClick={onConfirm}>Delete</button>
        </div>
      </div>
    </div>
  );
}
