// Session construction and the pure per-session reducers.
//
// Everything here is a plain function of (SessionState, args) -> Partial<SessionState>:
// no store access, no IPC. The store's bound actions in index.ts are thin wrappers
// that feed these into `updateSessionState`.

import { leaf, splitLeaf, removeLeaf, type SplitDir } from '../lib/layout';
import type { Task, Repo, Worktree } from '../types/ipc';
import type {
  EditorTab, OpenTabInput, SessionKind, SessionState, WorkspacePane,
} from './types';

/** Tab kinds that may exist at most once per session (one DOM/terminal instance):
 *  opening focuses the existing tab wherever it lives; splits never clone them. */
const UNIQUE_TAB_KINDS = new Set(['overview', 'terminal']);

/**
 * A pane that holds terminals holds nothing else.
 *
 * Terminals are a dock, not a document: they live in the bottom row, they never
 * mix with file tabs, and splitting one gives another terminal. Empty panes count
 * as neither, so a fresh split can still become either.
 */
export function isTerminalPane(pane: WorkspacePane): boolean {
  return pane.tabs.length > 0 && pane.tabs.every((t) => t.kind === 'terminal');
}

/** Where a tab of this kind is allowed to land. Keeps a file out of the terminal
 *  dock (and a terminal out of an editor pane) whatever the caller asks for. */
function paneFor(s: SessionState, kind: string, requested?: string): string {
  const wantsTerminal = kind === 'terminal';
  const pane = s.panes.find((p) => p.id === (requested ?? s.activePaneId));
  if (pane && isTerminalPane(pane) === wantsTerminal) return pane.id;
  const fallback = wantsTerminal
    ? s.panes.find((p) => isTerminalPane(p))
    : s.panes.find((p) => !isTerminalPane(p));
  return fallback?.id ?? pane?.id ?? s.activePaneId;
}

/** Commits fetched per page, and per scroll to the end of the list. */
export const COMMIT_PAGE = 20;

let paneSeq = 1;
export const newPaneId = () => `pane-${++paneSeq}`;
/** The task overview page — seeded as every session's first tab. */
export const overviewTab = (): EditorTab => ({
  id: '::overview', repoId: '', filePath: '', view: 'diff', kind: 'overview', label: 'Overview',
});

export const emptyPane = (): WorkspacePane => ({
  id: 'pane-1',
  tabs: [overviewTab()],
  activeTabId: '::overview',
});

/** Fresh per-session defaults. A factory (not a shared const) so every new
 *  session gets its own mutable containers (Sets, arrays, records). */
export function sessionDefaults(): Omit<SessionState, 'id' | 'kind' | 'title' | 'task' | 'worktrees' | 'repos'> {
  return {
    activeRepoId: null,
    panes: [emptyPane()],
    activePaneId: 'pane-1',
    layout: leaf('pane-1'),
    maximizedPaneId: null,
    editorFocusNonce: 0,
    sidebarTab: 'files',
    sidebarCollapsed: false,
    gitSubTab: 'changes',
    diff: null,
    diffHunks: {},
    diffMode: 'vs-main',
    diffNonce: 0,
    expandedDiffFiles: new Set<string>(),
    autoApprove: false,
    blameOn: false,
    blameByFile: {},
    worktreeStatus: {},
    commits: [],
    commitLimit: COMMIT_PAGE,
    commitsHasMore: true,
    annotations: [],
    mrs: [],
    mrThreadsByRepo: {},
    mrNonce: 0,
    rebaseConflict: null,
    ptySessions: [],
    activePtySessionId: null,
  };
}

let sessionSeq = 0;
const newSessionId = () => `sess-${++sessionSeq}`;

/** Tab label: explorers and reviews show their (human) title, tasks the short_id. */
export function sessionTitle(kind: SessionKind, task: Task | null): string {
  if (kind === 'explorer') return task?.title || 'Explorer';
  if (kind === 'review') return task?.title || 'Review';
  if (kind === 'desk') return 'Desk';
  return task?.short_id ?? 'Untitled';
}

export function newWorkspaceSession(kind: SessionKind, task: Task | null, worktrees: Worktree[], repos: Repo[]): SessionState {
  return {
    id: newSessionId(),
    kind,
    title: sessionTitle(kind, task),
    task,
    worktrees,
    repos,
    ...sessionDefaults(),
    activeRepoId: repos[0]?.id ?? null,
  };
}
let termTabSeq = 0;

export function openTabReducer(
  s: SessionState,
  { repoId, filePath, view, kind = 'file', sha, mrId, ptySessionId, label, cursorLine, preview: previewIn = false }: OpenTabInput,
  opts?: { paneId?: string }
): Partial<SessionState> {
  const paneId = paneFor(s, kind, opts?.paneId);
  const tabId =
    kind === 'changes' ? `${repoId}::__changes__`
    : kind === 'commit' ? `${repoId}::commit::${sha}`
    : kind === 'mr' ? `${repoId}::mr::${mrId}`
    : kind === 'overview' ? '::overview'
    // NEVER the label: every unbound terminal is labelled 'Terminal', so keying on
    // it made them all one tab and a second terminal silently focused the first.
    : kind === 'terminal' ? `::term::${ptySessionId ?? ++termTabSeq}`
    : `${repoId}::${filePath}`;
  // Single-instance surfaces never open as transient previews.
  const preview = UNIQUE_TAB_KINDS.has(kind) ? false : previewIn;

  // Single-instance kinds: if the tab exists in ANY pane, focus it there
  // (one DOM/terminal instance each — never a second copy). Terminals also
  // match by PTY-session binding: a tab bound to the same session IS the same
  // terminal even when its computed id differs (seq-based vs session-based),
  // otherwise a duplicate tab would steal the xterm host element.
  if (UNIQUE_TAB_KINDS.has(kind)) {
    for (const p of s.panes) {
      const existingUnique = p.tabs.find(
        (t) =>
          t.id === tabId ||
          (kind === 'terminal' && ptySessionId !== undefined && t.kind === 'terminal' && t.ptySessionId === ptySessionId)
      );
      if (existingUnique) {
        return {
          activePaneId: p.id,
          panes: s.panes.map((pp) => (pp.id === p.id ? { ...pp, activeTabId: existingUnique.id } : pp)),
        };
      }
    }
  }

  const panes = s.panes.map((p) => {
    if (p.id !== paneId) return p;
    const existing = p.tabs.find((t) => t.id === tabId);
    let tabs: EditorTab[];
    if (existing) {
      // Update in place. A normal open promotes a preview tab (clears the flag);
      // a preview open never demotes an already-open normal tab.
      tabs = p.tabs.map((t) =>
        t.id === tabId
          ? { ...t, view, cursorLine: cursorLine ?? t.cursorLine, preview: preview ? t.preview : false }
          : t
      );
    } else if (preview) {
      // Reuse the pane's single preview tab in place, else append a new one.
      const newTab: EditorTab = { id: tabId, repoId, filePath, view, kind, sha, mrId, ptySessionId, label, cursorLine, preview: true };
      const pIdx = p.tabs.findIndex((t) => t.preview);
      tabs = pIdx >= 0 ? p.tabs.map((t, i) => (i === pIdx ? newTab : t)) : [...p.tabs, newTab];
    } else {
      tabs = [...p.tabs, { id: tabId, repoId, filePath, view, kind, sha, mrId, ptySessionId, label, cursorLine }];
    }
    return { ...p, tabs, activeTabId: tabId };
  });
  return { panes, activePaneId: paneId };
}

/** Clear the preview flag on the pane's preview tab (keep it as a real tab). */
export function commitPreviewReducer(s: SessionState, paneId: string): Partial<SessionState> {
  const panes = s.panes.map((p) =>
    p.id === paneId ? { ...p, tabs: p.tabs.map((t) => (t.preview ? { ...t, preview: false } : t)) } : p
  );
  return { panes };
}

/** Remove the pane's preview tab. Unlike closeTab, never prunes an empty split
 *  pane — discarding a preview shouldn't collapse a split the user created. */
export function discardPreviewReducer(s: SessionState, paneId: string): Partial<SessionState> {
  const panes = s.panes.map((p) => {
    if (p.id !== paneId) return p;
    const idx = p.tabs.findIndex((t) => t.preview);
    if (idx < 0) return p;
    const removedId = p.tabs[idx].id;
    const tabs = p.tabs.filter((t) => !t.preview);
    const activeTabId =
      p.activeTabId === removedId ? (tabs.length ? tabs[Math.max(0, idx - 1)].id : null) : p.activeTabId;
    return { ...p, tabs, activeTabId };
  });
  return { panes };
}

export function closeTabReducer(s: SessionState, paneId: string, tabId: string): Partial<SessionState> {
  const panes = s.panes.map((p) => {
    if (p.id !== paneId) return p;
    const idx = p.tabs.findIndex((t) => t.id === tabId);
    const tabs = p.tabs.filter((t) => t.id !== tabId);
    const activeTabId =
      p.activeTabId === tabId ? (tabs.length ? tabs[Math.max(0, idx - 1)].id : null) : p.activeTabId;
    return { ...p, tabs, activeTabId };
  });
  // Closing the last tab of a non-root pane closes the pane (tree collapses).
  const emptied = panes.find((p) => p.id === paneId && p.tabs.length === 0);
  if (emptied && panes.length > 1) {
    return closePaneReducer({ ...s, panes }, paneId);
  }
  return { panes };
}

/**
 * Split the active pane in `dir`; the new pane clones the active tab (except
 * single-instance kinds, which can only exist once) and takes focus.
 *
 * Splitting a terminal pane is the exception: it always yields another terminal,
 * side by side in the dock, because a terminal pane can hold nothing else and an
 * empty half of the dock is useless. The caller starts the PTY for the new tab.
 */
export function splitPaneReducer(s: SessionState, dir: SplitDir): Partial<SessionState> {
  const active = s.panes.find((p) => p.id === s.activePaneId) ?? s.panes[0];
  if (!active) return {};
  const id = newPaneId();

  if (isTerminalPane(active)) {
    const tab: EditorTab = {
      id: `::term::${++termTabSeq}`,
      repoId: '', filePath: '', view: 'diff', kind: 'terminal', label: 'Terminal',
    };
    return {
      panes: [...s.panes, { id, tabs: [tab], activeTabId: tab.id }],
      activePaneId: id,
      layout: splitLeaf(s.layout, active.id, 'row', id),
    };
  }

  const activeTab = active.tabs.find((t) => t.id === active.activeTabId) ?? null;
  const clone = activeTab && !UNIQUE_TAB_KINDS.has(activeTab.kind ?? 'file') ? { ...activeTab, preview: false } : null;
  const pane: WorkspacePane = clone
    ? { id, tabs: [clone], activeTabId: clone.id }
    : { id, tabs: [], activeTabId: null };
  return {
    panes: [...s.panes, pane],
    activePaneId: id,
    layout: splitLeaf(s.layout, active.id, dir, id),
  };
}

/** Close a pane: merge its tabs into the surviving sibling (dedupe by id — PTY
 *  and other single-instance tabs are preserved), collapse the tree. */
export function closePaneReducer(s: SessionState, paneId: string): Partial<SessionState> {
  if (s.panes.length <= 1) return {};
  const closing = s.panes.find((p) => p.id === paneId);
  if (!closing) return {};
  const [layout, survivorId] = removeLeaf(s.layout, paneId);
  if (layout === s.layout || !survivorId) return {};
  const panes = s.panes
    .filter((p) => p.id !== paneId)
    .map((p) => {
      if (p.id !== survivorId) return p;
      const merged = [...p.tabs];
      for (const t of closing.tabs) {
        if (!merged.some((m) => m.id === t.id)) merged.push({ ...t, preview: false });
      }
      return { ...p, tabs: merged, activeTabId: p.activeTabId ?? merged[0]?.id ?? null };
    });
  return {
    panes,
    layout,
    activePaneId: s.activePaneId === paneId ? survivorId : s.activePaneId,
    maximizedPaneId: s.maximizedPaneId === paneId ? null : s.maximizedPaneId,
  };
}

/** Force a diff reload: bump the nonce (re-fetches the summary) and clear cached
 *  hunks (re-fetches expanded files). Shared by session.bumpDiff + root.invalidateDiff.
 *  Blame goes too — a commit re-attributes lines. */
export const bumpDiffRecipe = (s: SessionState): Partial<SessionState> => ({
  diffNonce: s.diffNonce + 1,
  diffHunks: {},
  blameByFile: {},
});
