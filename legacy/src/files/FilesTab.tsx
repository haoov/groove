import { Search, TextSearch, X } from 'lucide-react';
import { useSession } from '../shared/store';
import type { Worktree } from '../shared/ipc/ipc';
import { FileTreeNodes } from './tree';
import { TreeContextMenu, TreePrompt, TreeConfirmDelete } from './FileTreeMenu';
import { useFileSearch } from './useFileSearch';
import { useFileTree } from './useFileTree';
import { useTreeOps } from './useTreeOps';
import { GrepResults, NameResults } from './SearchResults';

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
  const wt = repoId ? worktreeForRepo(repoId) : undefined;
  const wtPath = wt?.path;

  const tree = useFileTree({ repoId, wtPath, expandedDirs, onToggleDir, onOpenFile });
  const ops = useTreeOps({
    repoId, wtPath, files: tree.files, expandedDirs, onToggleDir, reload: tree.loadFiles,
  });
  const search = useFileSearch({ repoId, wtPath, files: tree.files });
  const { mode, query, searching } = search;
  const { prompt } = ops;

  if (!repoId) return <div className="sidebar-empty">Select a repo above</div>;
  if (!wt) return <div className="sidebar-empty">No active worktree</div>;

  const changedPaths = new Set(
    (diff?.repos.find((r) => r.worktree_id === wt.id) ?? diff?.repos.find((r) => r.repo_id === repoId))?.files.map((f) => f.path) ?? []
  );

  return (
    <div className="files-panel">
      <div className="file-search">
        <Search className="file-search-icon" size={14} strokeWidth={2} />
        <input
          ref={search.inputRef}
          data-file-search="1"
          className="file-search-input"
          placeholder={mode === 'text' ? 'Search in files…' : 'Find file…'}
          value={query}
          onChange={(e) => search.onQueryChange(e.target.value)}
          onKeyDown={search.onInputKeyDown}
          spellCheck={false}
        />
        <div className="file-search-modes" role="group" aria-label="Search mode">
          <button
            className={`file-search-mode ${mode === 'name' ? 'active' : ''}`}
            title="Find file by name"
            onMouseDown={(e) => { e.preventDefault(); search.switchMode('name'); }}
          >
            <Search size={13} strokeWidth={2} />
          </button>
          <button
            className={`file-search-mode ${mode === 'text' ? 'active' : ''}`}
            title="Search file contents"
            onMouseDown={(e) => { e.preventDefault(); search.switchMode('text'); }}
          >
            <TextSearch size={13} strokeWidth={2} />
          </button>
        </div>
        {searching && (
          <button className="file-search-clear" title="Clear (Esc)" onMouseDown={(e) => { e.preventDefault(); search.cancelSearch(); }}>
            <X size={13} strokeWidth={2} />
          </button>
        )}
      </div>

      {tree.loadingFiles ? (
        <div className="sidebar-empty">Loading…</div>
      ) : searching && mode === 'text' ? (
        <GrepResults search={search} />
      ) : searching ? (
        <NameResults search={search} />
      ) : tree.tree.length === 0 ? (
        <div className="sidebar-empty">No files</div>
      ) : (
        <div
          className="files-list nav-list"
          tabIndex={0}
          ref={tree.nav.containerRef}
          onKeyDown={tree.nav.onKeyDown}
          onContextMenu={(e) => { e.preventDefault(); ops.setMenu({ x: e.clientX, y: e.clientY, node: null }); }}
        >
          <FileTreeNodes
            nodes={tree.tree}
            depth={0}
            modifiedPaths={changedPaths}
            repoId={repoId}
            expandedDirs={expandedDirs}
            onToggleDir={onToggleDir}
            onOpenFile={onOpenFile}
            // Tree files have no diff view.
            onOpenFileAlt={onOpenFile}
            selectedPath={tree.selectedPath}
            onSelect={tree.selectRow}
            onContextMenu={(node, e) => ops.setMenu({ x: e.clientX, y: e.clientY, node })}
          />
        </div>
      )}

      {ops.menu && (
        <TreeContextMenu
          x={ops.menu.x} y={ops.menu.y} node={ops.menu.node} hasClipboard={!!ops.clipboard}
          onAction={ops.onMenuAction} onClose={() => ops.setMenu(null)}
        />
      )}
      {prompt && (
        <TreePrompt
          title={prompt.title} initialValue={prompt.initial} confirmLabel={prompt.confirmLabel}
          onSubmit={(v) => { prompt.run(v); ops.setPrompt(null); }} onCancel={() => ops.setPrompt(null)}
        />
      )}
      {ops.confirmDel && (
        <TreeConfirmDelete node={ops.confirmDel} onConfirm={ops.doDelete} onCancel={() => ops.setConfirmDel(null)} />
      )}
    </div>
  );
}
