import { useEffect, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { X, GitCompare, Code2, Columns2, Rows2, Maximize2, Minimize2, GitCommit, Terminal as TerminalIcon, Skull } from 'lucide-react';
import { useStore, useSession, type EditorTab } from '../shared/store';
import { shortcutLabel } from '../shared/lib/keybindings';
import { worktreeFor, activeWorktreeFor, mrForWorktree, openFileAnnotations, fileThreads } from '../shared/lib/workspace';
import { ensureTerminalTab } from '../shared/lib/panes';
import { useDiffExpand } from '../editor/useDiffExpand';
import { useBlame } from '../editor/useBlame';
import type { Hunk } from '../shared/ipc/ipc';
import { guessLang } from '../shared/lib/lang';
import { FileDiffEditor } from '../editor/FileDiffEditor';
import { ChangesView } from '../git/DiffView';
import { CommitDiffView } from '../git/CommitDiffView';
import { PtyTabBody } from '../terminal/PtyTabBody';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { CodeEditor } from '../editor/CodeEditor';
import { killPtyTab } from '../shared/lib/panes';
import type { AnnCtx } from '../editor/useAnnotations';

const isPtyKind = (k?: EditorTab['kind']) => k === 'terminal';

function tabLabel(tab: EditorTab): string {
  if (tab.kind === 'changes') return 'All changes';
  if (tab.kind === 'commit') return tab.label ?? tab.sha?.slice(0, 7) ?? 'commit';
  if (tab.kind === 'terminal') return tab.label ?? 'Terminal';
  return tab.filePath.split('/').pop() ?? tab.filePath;
}

// ── One pane: a tab strip over the active tab's content ───────────────────────

export function WorkspacePane({
  pane, ann, isActive, multiRepo,
}: {
  pane: import('../shared/store').WorkspacePane;
  ann: AnnCtx;
  isActive: boolean;
  multiRepo: boolean;
}) {
  const activeRepos = useSession((s) => s.activeRepos);
  const focusPane = useSession((s) => s.focusPane);
  const setActiveTab = useSession((s) => s.setActiveTab);
  const closeTab = useSession((s) => s.closeTab);
  const setTabView = useSession((s) => s.setTabView);
  const editorFocusNonce = useSession((s) => s.editorFocusNonce);
  const panes = useSession((s) => s.panes);
  const splitPane = useSession((s) => s.splitPane);
  const closePane = useSession((s) => s.closePane);
  const maximizedPaneId = useSession((s) => s.maximizedPaneId);
  const toggleMaximizePane = useSession((s) => s.toggleMaximizePane);
  const multiPane = panes.length >= 2;
  const isMaximized = maximizedPaneId === pane.id;

  const activeTab = pane.tabs.find((t) => t.id === pane.activeTabId) ?? null;
  // Only the active pane's editor reacts to the focus nonce, so a background
  // pane never steals focus on an open/commit elsewhere.
  const focusSignal = isActive ? editorFocusNonce : undefined;
  const repoName = (repoId: string) => activeRepos.find((r) => r.id === repoId)?.project ?? repoId;

  // Right-click menu on any tab: close / close others / left / right / all, plus
  // Kill on a terminal (its plain close only hides — the session keeps running).
  const sessionKey = useSession((s) => s.id);
  const [tabMenu, setTabMenu] = useState<{ x: number; y: number; tab: EditorTab } | null>(null);

  const closeMany = (ids: string[]) => { setTabMenu(null); ids.forEach((id) => closeTab(pane.id, id)); };

  return (
    <div
      className={`ws-pane ${isActive ? 'active' : ''}`}
      onMouseDownCapture={() => { if (!isActive) focusPane(pane.id); }}
    >
      <div className="ws-tabstrip">
        <div className="ws-tabs">
          {pane.tabs.map((t) => (
            <div
              key={t.id}
              className={`ws-tab ${t.id === pane.activeTabId ? 'active' : ''} ${t.preview ? 'preview' : ''}`}
              onMouseDown={(e) => { if (e.button === 0) setActiveTab(pane.id, t.id); }}
              onAuxClick={(e) => { if (e.button === 1) { e.preventDefault(); closeTab(pane.id, t.id); } }}
              onContextMenu={(e) => { e.preventDefault(); setTabMenu({ x: e.clientX, y: e.clientY, tab: t }); }}
              title={
                t.kind === 'changes' ? `${repoName(t.repoId)} — all changes`
                : t.kind === 'commit' ? `${repoName(t.repoId)} · commit ${t.label ?? t.sha?.slice(0, 7) ?? ''}`
                : t.kind === 'terminal' ? 'Terminal — close hides; right-click for options'
                : `${repoName(t.repoId)} · ${t.filePath}`
              }
            >
              {t.kind === 'commit' && <GitCommit size={11} strokeWidth={1.75} className="ws-tab-icon" />}
              {t.kind === 'terminal' && <TerminalIcon size={11} strokeWidth={1.75} className="ws-tab-icon" />}
              {(t.kind === 'file' || !t.kind) && (
                t.view === 'diff'
                  ? <GitCompare size={11} strokeWidth={1.75} className="ws-tab-icon" />
                  : <Code2 size={11} strokeWidth={1.75} className="ws-tab-icon" />
              )}
              <span className={`ws-tab-labels ${multiRepo ? 'multi' : ''}`}>
                <span className="ws-tab-name">{tabLabel(t)}</span>
                {multiRepo && <span className="ws-tab-repo">{repoName(t.repoId)}</span>}
              </span>
              <button
                className="ws-tab-close"
                title="Close"
                // Left button only — right-clicking the × must not close the tab.
                onMouseDown={(e) => { if (e.button !== 0) return; e.stopPropagation(); closeTab(pane.id, t.id); }}
                onContextMenu={(e) => { e.preventDefault(); e.stopPropagation(); }}
              >
                <X size={11} strokeWidth={2.25} />
              </button>
            </div>
          ))}
        </div>

        <div className="ws-view-toggle">
          {/* Per-tab Diff⇄Edit toggle (file tabs only). */}
          {activeTab && (activeTab.kind === 'file' || !activeTab.kind) && (
            <>
              <button
                className={`ws-view-btn ${activeTab.view === 'diff' ? 'active' : ''}`}
                onClick={() => setTabView(pane.id, activeTab.id, 'diff')}
                title="View as diff"
              >
                <GitCompare size={12} strokeWidth={1.75} />
              </button>
              <button
                className={`ws-view-btn ${activeTab.view === 'edit' ? 'active' : ''}`}
                onClick={() => setTabView(pane.id, activeTab.id, 'edit')}
                title="Edit file"
              >
                <Code2 size={12} strokeWidth={1.75} />
              </button>
              <span className="ws-toggle-sep" />
            </>
          )}
          {/* Pane controls live in the tab band, Zed-style. */}
          <button
            className="ws-view-btn"
            onClick={() => splitPane('row')}
            title="Split right (Ctrl+\)"
          >
            <Columns2 size={12} strokeWidth={1.75} />
          </button>
          <button
            className="ws-view-btn"
            onClick={() => splitPane('col')}
            title="Split down (Ctrl+Shift+\)"
          >
            <Rows2 size={12} strokeWidth={1.75} />
          </button>
          {multiPane && (
            <>
              <button
                className={`ws-view-btn ${isMaximized ? 'active' : ''}`}
                onClick={toggleMaximizePane}
                title={isMaximized ? 'Restore panes' : 'Maximize pane'}
              >
                {isMaximized
                  ? <Minimize2 size={12} strokeWidth={1.75} />
                  : <Maximize2 size={12} strokeWidth={1.75} />}
              </button>
              <button
                className="ws-view-btn"
                onClick={() => closePane(pane.id)}
                title="Close pane (tabs move to the neighbor)"
              >
                <X size={12} strokeWidth={1.75} />
              </button>
            </>
          )}
        </div>
      </div>

      {activeTab && (activeTab.kind === 'file' || !activeTab.kind) && (
        <Breadcrumbs repoId={activeTab.repoId} filePath={activeTab.filePath} />
      )}

      <div className="ws-pane-body">
        {/* PTY tabs stay mounted (hidden when inactive): the xterm host element
            must never remount, and warm mounts avoid refit flashes. */}
        {pane.tabs.filter((t) => isPtyKind(t.kind)).map((t) => (
          <div
            key={t.id}
            className="ws-pty-host"
            style={{ display: t.id === pane.activeTabId ? 'flex' : 'none' }}
          >
            <PtyTabBody tab={t} paneId={pane.id} isActive={t.id === pane.activeTabId && isActive} />
          </div>
        ))}
        {!activeTab ? (
          <div className="ws-pane-empty">
            <p>No file open</p>
            <p className="editor-hint"><kbd>Ctrl+P</kbd> find file · click a changed file in the sidebar</p>
            {/* A split leaves an empty pane, and a terminal is the usual reason
                for making one. */}
            <button className="btn-secondary" onClick={() => ensureTerminalTab()}>
              <TerminalIcon size={12} strokeWidth={1.75} style={{ marginRight: 6 }} />
              New terminal
            </button>
          </div>
        ) : isPtyKind(activeTab.kind) ? null : activeTab.kind === 'changes' ? (
          <ChangesView repoId={activeTab.repoId} ann={ann} />
        ) : activeTab.kind === 'commit' ? (
          <CommitDiffView repoId={activeTab.repoId} sha={activeTab.sha!} ann={ann} />
        ) : activeTab.view === 'diff' ? (
          <DiffTab tab={activeTab} ann={ann} focusSignal={focusSignal} />
        ) : (
          <EditTab tab={activeTab} ann={ann} focusSignal={focusSignal} />
        )}
      </div>

      {tabMenu && (() => {
        const ids = pane.tabs.map((t) => t.id);
        const i = ids.indexOf(tabMenu.tab.id);
        const others = ids.filter((id) => id !== tabMenu.tab.id);
        const left = ids.slice(0, i);
        const right = ids.slice(i + 1);
        return (
          <ContextMenu x={tabMenu.x} y={tabMenu.y} onClose={() => setTabMenu(null)} className="ctx-menu">
            <button className="ctx-menu-item" onClick={() => closeMany([tabMenu.tab.id])}>
              <X size={13} strokeWidth={1.75} style={{ marginRight: 7 }} />
              Close
            </button>
            <button className="ctx-menu-item" disabled={others.length === 0} onClick={() => closeMany(others)}>
              Close others
            </button>
            <button className="ctx-menu-item" disabled={right.length === 0} onClick={() => closeMany(right)}>
              Close to the right
            </button>
            <button className="ctx-menu-item" disabled={left.length === 0} onClick={() => closeMany(left)}>
              Close to the left
            </button>
            <button className="ctx-menu-item" onClick={() => closeMany(ids)}>
              Close all
            </button>
            {isPtyKind(tabMenu.tab.kind) && (
              <>
                <div className="ctx-menu-sep" />
                <button
                  className="ctx-menu-item ctx-menu-item--danger"
                  onClick={() => { killPtyTab(sessionKey, pane.id, tabMenu.tab); setTabMenu(null); }}
                >
                  <Skull size={13} strokeWidth={1.75} style={{ marginRight: 7 }} />
                  Kill terminal session
                </button>
              </>
            )}
          </ContextMenu>
        );
      })()}
    </div>
  );
}

// ── Diff tab: one file's diff (lazy-loads its hunks) ──────────────────────────

function DiffTab({ tab, ann, focusSignal }: { tab: EditorTab; ann: AnnCtx; focusSignal?: number }) {
  const diffHunks = useSession((s) => s.diffHunks);
  const setDiffHunks = useSession((s) => s.setDiffHunks);
  const diffMode = useSession((s) => s.diffMode);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const annotations = useSession((s) => s.annotations);
  const mrThreadsByRepo = useSession((s) => s.mrThreadsByRepo);
  const mrs = useSession((s) => s.mrs);
  const setLastError = useStore((s) => s.setLastError);
  const inFlight = useRef(false);

  const activeWorktreeId = useSession((s) => s.activeWorktreeId);
  const wt = activeWorktreeFor(activeWorktrees, tab.repoId, activeWorktreeId);
  // Worktree-keyed (falls back to the repo while provisioning): one repo can have
  // several worktrees in a session, so a repo key would collide across branches.
  const key = `${wt?.id ?? tab.repoId}/${tab.filePath}`;
  const hunks = diffHunks[key];
  const expand = useDiffExpand({
    worktreeId: wt?.id, filePath: tab.filePath, hunks,
    onHunks: (h) => setDiffHunks(key, h),
  });
  const { blameOn, blame, openCommit } = useBlame({
    worktreeId: wt?.id, repoId: tab.repoId, filePath: tab.filePath,
  });

  useEffect(() => {
    if (!wt || hunks !== undefined || inFlight.current) return;
    inFlight.current = true;
    invoke<Hunk[]>('get_file_diff', { worktreeId: wt.id, filePath: tab.filePath, mode: diffMode })
      .then((h) => setDiffHunks(key, h))
      .catch((e) => setLastError(String(e)))
      .finally(() => { inFlight.current = false; });
  }, [key, wt, hunks, diffMode, tab.filePath, setDiffHunks, setLastError]);

  if (!wt) return <div className="diff-empty"><p>No worktree for this repo</p></div>;

  const fileAnns = openFileAnnotations(annotations, tab.repoId, tab.filePath);
  const threads = mrThreadsByRepo[tab.repoId] ?? [];
  const mr = mrForWorktree(mrs, wt.id);

  return (
    <div className="diff-view" onClick={() => ann.cancel()}>
      {hunks === undefined ? (
        <div className="diff-file-loading">Loading diff…</div>
      ) : hunks.length === 0 ? (
        <div className="diff-empty"><p>No text changes (or binary file)</p></div>
      ) : (
        <FileDiffEditor
          hunks={hunks}
          filePath={tab.filePath}
          repoId={tab.repoId}
          ann={ann}
          sel={ann.sel}
          dragRange={ann.dragRange}
          fileAnnotations={fileAnns}
          threads={threads}
          mr={mr}
          focusSignal={focusSignal}
          isPreview={tab.preview}
          onExpandGap={expand.onExpand}
          fileLineCount={expand.total}
          blame={blameOn ? blame : undefined}
          onOpenCommit={openCommit}
        />
      )}
    </div>
  );
}

// ── Edit tab: an editable CodeMirror buffer for one file ──────────────────────

function EditTab({ tab, ann, focusSignal }: { tab: EditorTab; ann: AnnCtx; focusSignal?: number }) {
  const blameHint = useStore((s) => shortcutLabel(s.keymap, 'editor.toggleBlame'));
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const annotations = useSession((s) => s.annotations);
  const mrThreadsByRepo = useSession((s) => s.mrThreadsByRepo);
  const mrs = useSession((s) => s.mrs);

  const [modified, setModified] = useState(false);
  const saveFnRef = useRef<() => void>(() => {});

  const wt = worktreeFor(activeWorktrees, tab.repoId);
  const languageId = guessLang(tab.filePath);
  const { blameOn, blame, openCommit } = useBlame({
    worktreeId: wt?.id, repoId: tab.repoId, filePath: tab.filePath,
  });
  const setBlameOn = useSession((s) => s.setBlameOn);

  // Tell the backend this is the current open file (cursor-persistence target).
  useEffect(() => {
    if (!wt || !activeTask) return;
    invoke('open_file', { taskId: activeTask.short_id, repoId: tab.repoId, filePath: tab.filePath, languageId }).catch(console.error);
  }, [wt?.id, activeTask?.short_id, tab.repoId, tab.filePath, languageId]);

  if (!wt || !activeTask) return <div className="diff-empty"><p>No worktree for this repo</p></div>;

  const fileAnns = openFileAnnotations(annotations, tab.repoId, tab.filePath);
  const threadsForFile = fileThreads(mrThreadsByRepo[tab.repoId] ?? [], tab.filePath);
  const mr = mrForWorktree(mrs, wt.id);

  const onSaveContent = async (content: string) => {
    await invoke('save_file', { worktreePath: wt.path, filePath: tab.filePath, content });
  };
  const onPersistCursor = (cursorLine: number, cursorCol: number, scrollTop: number) => {
    invoke('update_open_file_state', { cursorLine, cursorCol, scrollTop }).catch(console.error);
  };

  return (
    <div className="edit-tab">
      <CodeEditor
        worktreePath={wt.path}
        filePath={tab.filePath}
        repoId={tab.repoId}
        languageId={languageId}
        initialCursorLine={tab.cursorLine ?? 0}
        initialCursorCol={0}
        annotations={fileAnns}
        threads={threadsForFile}
        mr={mr}
        ann={ann}
        onModifiedChange={setModified}
        onPersistCursor={onPersistCursor}
        onSaveContent={onSaveContent}
        registerSave={(fn) => { saveFnRef.current = fn; }}
        focusSignal={focusSignal}
        isPreview={tab.preview}
        blame={blameOn ? blame : undefined}
        onOpenCommit={openCommit}
      />
      <div className="edit-tab-foot">
        <span className="editor-lang">{languageId}</span>
        <span className="editor-path">{tab.filePath}</span>
        {modified && <span className="editor-modified">●</span>}
        <button
          className={`editor-foot-btn ${blameOn ? 'on' : ''}`}
          onClick={() => setBlameOn(!blameOn)}
          title={`Blame — who last changed each line${blameHint ? ` (${blameHint})` : ''}`}
        >
          blame
        </button>
        <button className="editor-save-btn" onClick={() => saveFnRef.current()} title="Save (Ctrl+S)">Save</button>
      </div>
    </div>
  );
}

/**
 * Where the open file lives, above the editor.
 *
 * Directory segments expand the tree down to themselves (store `revealInTree`);
 * the file segment is inert — you are already looking at it. Only file tabs get
 * one: diffs, terminals and the overview have no path to show.
 */
function Breadcrumbs({ repoId, filePath }: { repoId: string; filePath: string }) {
  const repos = useSession((s) => s.activeRepos);
  const setSidebarTab = useSession((s) => s.setSidebarTab);
  const revealInTree = useStore((s) => s.revealInTree);
  if (!filePath) return null;

  const repo = repos.find((r) => r.id === repoId);
  const parts = filePath.split('/').filter(Boolean);

  const reveal = (dir: string) => {
    setSidebarTab('files');
    revealInTree(dir);
  };

  return (
    <div className="ws-crumbs" title={filePath}>
      {repo && <span className="ws-crumb-repo">{repo.project}</span>}
      {parts.map((part, i) => {
        const isLast = i === parts.length - 1;
        const dir = parts.slice(0, i + 1).join('/');
        return (
          <span key={dir} className="ws-crumb-part">
            <span className="ws-crumb-sep">›</span>
            {isLast ? (
              <span className="ws-crumb-file">{part}</span>
            ) : (
              <button className="ws-crumb-dir" onClick={() => reveal(dir)}>{part}</button>
            )}
          </span>
        );
      })}
    </div>
  );
}
