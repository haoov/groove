import { invoke } from '../../ipc/invoke';
import type { StateCreator, StoreApi } from 'zustand';
import type { WorktreeStatus } from '../../ipc/ipc';
import {
  splitRoot, leafOrder, setRatio as setLayoutRatio,
} from '../../lib/layout';
import type { AppState, SessionActions, SessionState, SessionView, SessionsSlice } from '../types';
import {
  COMMIT_PAGE, bumpDiffRecipe, closePaneReducer, closeTabReducer, commitPreviewReducer,
  discardPreviewReducer, newPaneId, newWorkspaceSession, openTabReducer, sessionDefaults,
  sessionTitle, splitPaneReducer,
} from '../session';

// The slice owns the whole session layer: the map + lifecycle actions, the
// per-session bound-action machinery, and the memoized `SessionView` builder.
// Module-level helpers need the live store; the creator captures set/get once
// (the store is created before anything can call them).

let _set: StoreApi<AppState>['setState'];
let _get: StoreApi<AppState>['getState'];

export const sessionsSlice: StateCreator<AppState, [], [], SessionsSlice> = (set, get) => {
  _set = set as StoreApi<AppState>['setState'];
  _get = get;
  return {
    sessions: {},
    sessionOrder: [],
    activeSessionId: null,
    openSession: ({ kind, task = null, worktrees = [], repos = [], focus = true }) => {
      const st = get();
      // Navigation is opt-out: only a focusing open moves the user.
      const navigate = focus ? { view: 'workspace' as const } : {};
      // Dedupe by short_id — reopening a task/explorer focuses its session.
      if (task) {
        const existingId = st.sessionOrder.find((id) => st.sessions[id]?.task?.short_id === task.short_id);
        if (existingId) {
          set((s) => {
            const prev = s.sessions[existingId];
            return {
              sessions: {
                ...s.sessions,
                [existingId]: {
                  ...prev,
                  task,
                  worktrees,
                  repos,
                  // Preserve the existing session's kind; only refresh the label.
                  title: sessionTitle(prev.kind, task),
                  activeRepoId: prev.activeRepoId ?? repos[0]?.id ?? null,
                  // Keep the selected worktree if it survived the refresh
                  // (conversion/provisioning replace worktree rows).
                  activeWorktreeId: worktrees.some((w) => w.id === prev.activeWorktreeId)
                    ? prev.activeWorktreeId
                    : worktrees.find((w) => w.repo_id === (prev.activeRepoId ?? repos[0]?.id))?.id
                      ?? worktrees[0]?.id ?? null,
                },
              },
              ...navigate,
              ...(focus ? { activeSessionId: existingId } : {}),
            };
          });
          return existingId;
        }
      }
      const sess = newWorkspaceSession(kind, task, worktrees, repos);
      set((s) => ({
        sessions: { ...s.sessions, [sess.id]: sess },
        sessionOrder: [...s.sessionOrder, sess.id],
        ...navigate,
        ...(focus ? { activeSessionId: sess.id } : {}),
      }));
      return sess.id;
    },
    focusSession: (id) => set({ activeSessionId: id, view: 'workspace' }),
    closeSession: (id) =>
      set((s) => {
        if (!s.sessions[id]) return {};
        const sessionOrder = s.sessionOrder.filter((x) => x !== id);
        const sessions = { ...s.sessions };
        delete sessions[id];
        boundActionsCache.delete(id);
        let activeSessionId = s.activeSessionId;
        let view = s.view;
        if (s.activeSessionId === id) {
          const at = s.sessionOrder.indexOf(id);
          const remaining = s.sessionOrder.filter((x) => x !== id);
          activeSessionId = remaining[at] ?? remaining[at - 1] ?? null;
          // Only the workspace has nothing left to show; settings stays put.
          if (!activeSessionId && view === 'workspace') view = 'home';
        }
        return { sessions, sessionOrder, activeSessionId, view };
      }),
    updateSession: (id, patch) =>
      updateSessionState(id, typeof patch === 'function' ? patch : () => patch),
    refreshStatusFor: (id) => doRefreshStatus(id),
    invalidateDiff: (id) => updateSessionState(id, bumpDiffRecipe),
    invalidateMrs: (id) => updateSessionState(id, (s) => ({ mrNonce: s.mrNonce + 1 })),
  };
};

// ─── Session state mutation + bound actions ─────────────────────────────────────

/** Immutably patch one session, merging only the changed fields. */
function updateSessionState(id: string, recipe: (s: SessionState) => Partial<SessionState>) {
  if (!id) return;
  _set((st) => {
    const sess = st.sessions[id];
    if (!sess) return {};
    return { sessions: { ...st.sessions, [id]: { ...sess, ...recipe(sess) } } };
  });
}

/** Recompute git status for a session's active worktrees. Shared by the bound
 *  per-session action and the root `refreshStatusFor` (used by event handlers). */
// One run per session at a time — overlapping callers fold into a trailing
// re-run instead of stacking N-per-worktree git calls.
const statusInFlight = new Set<string>();
const statusQueued = new Set<string>();

async function doRefreshStatus(id: string) {
  if (statusInFlight.has(id)) {
    statusQueued.add(id);
    return;
  }
  const sess = _get().sessions[id];
  if (!sess) return;
  statusInFlight.add(id);
  try {
    const entries = await Promise.allSettled(
      sess.worktrees.map((w) => invoke<WorktreeStatus>('get_worktree_status', { worktreeId: w.id }))
    );
    const next: Record<string, WorktreeStatus> = {};
    for (const r of entries) {
      if (r.status === 'fulfilled') next[r.value.worktree_id] = r.value;
    }
    updateSessionState(id, () => ({ worktreeStatus: next }));
  } finally {
    statusInFlight.delete(id);
    if (statusQueued.delete(id)) void doRefreshStatus(id);
  }
}

/** Per-session actions, bound to a session id. Cached so references stay stable. */
function makeSessionActions(id: string): SessionActions {
  const upd = (recipe: (s: SessionState) => Partial<SessionState>) => updateSessionState(id, recipe);
  return {
    setWorkspaceMode: (m) => upd(() => ({ workspaceMode: m })),
    setDiffMode: (m) => upd(() => ({ diffMode: m })),
    bumpDiff: () => upd(bumpDiffRecipe),
    refreshStatus: () => doRefreshStatus(id),
    splitPane: (dir) => upd((s) => splitPaneReducer(s, dir)),
    splitRootPane: (dir, ratio = 0.5) =>
      upd((s) => {
        const paneId = newPaneId();
        return {
          panes: [...s.panes, { id: paneId, tabs: [], activeTabId: null }],
          layout: splitRoot(s.layout, dir, paneId, ratio),
          activePaneId: paneId,
        };
      }),
    closePane: (paneId) => upd((s) => closePaneReducer(s, paneId)),
    setSplitRatio: (splitId, ratio) => upd((s) => ({ layout: setLayoutRatio(s.layout, splitId, ratio) })),
    toggleMaximizePane: () =>
      upd((s) => ({ maximizedPaneId: s.maximizedPaneId === s.activePaneId ? null : s.activePaneId })),
    focusNextPane: () =>
      upd((s) => {
        const order = leafOrder(s.layout);
        if (order.length < 2) return {};
        const idx = order.indexOf(s.activePaneId);
        return { activePaneId: order[(idx + 1) % order.length] };
      }),
    openTab: (input, opts) =>
      upd((s) => {
        const patch = openTabReducer(s, input, opts);
        // Any open shows the panes: leaving Overview is implied by asking for a tab.
        // A real open focuses the editor; a preview open leaves focus in the search input.
        return input.preview
          ? { ...patch, workspaceMode: 'code' as const }
          : { ...patch, workspaceMode: 'code' as const, editorFocusNonce: s.editorFocusNonce + 1 };
      }),
    commitPreview: (paneId) =>
      upd((s) => ({ ...commitPreviewReducer(s, paneId), editorFocusNonce: s.editorFocusNonce + 1 })),
    discardPreview: (paneId) => upd((s) => discardPreviewReducer(s, paneId)),
    closeTab: (paneId, tabId) => upd((s) => closeTabReducer(s, paneId, tabId)),
    setActiveTab: (paneId, tabId) =>
      upd((s) => ({
        activePaneId: paneId,
        panes: s.panes.map((p) => (p.id === paneId ? { ...p, activeTabId: tabId } : p)),
      })),
    setTabView: (paneId, tabId, view) =>
      upd((s) => ({
        panes: s.panes.map((p) =>
          p.id === paneId ? { ...p, tabs: p.tabs.map((t) => (t.id === tabId ? { ...t, view } : t)) } : p
        ),
      })),
    setTabPty: (paneId, tabId, ptySessionId) =>
      upd((s) => ({
        panes: s.panes.map((p) =>
          p.id === paneId ? { ...p, tabs: p.tabs.map((t) => (t.id === tabId ? { ...t, ptySessionId } : t)) } : p
        ),
      })),
    focusPane: (paneId) => upd(() => ({ activePaneId: paneId })),
    setActiveRepoId: (rid) =>
      upd((s) => {
        const current = s.worktrees.find((w) => w.id === s.activeWorktreeId);
        const wt = current?.repo_id === rid ? current : s.worktrees.find((w) => w.repo_id === rid);
        return { activeRepoId: rid, activeWorktreeId: wt?.id ?? null };
      }),
    setActiveWorktreeId: (wid) =>
      upd((s) => {
        const wt = s.worktrees.find((w) => w.id === wid);
        return { activeWorktreeId: wid, activeRepoId: wt?.repo_id ?? s.activeRepoId };
      }),
    setSidebarTab: (t) => upd(() => ({ sidebarTab: t })),
    setSidebarCollapsed: (v) => upd(() => ({ sidebarCollapsed: v })),
    setGitSubTab: (t) => upd(() => ({ gitSubTab: t })),
    setDiff: (d) => upd(() => ({ diff: d, diffHunks: {} })),
    setDiffHunks: (key, hunks) => upd((s) => ({ diffHunks: { ...s.diffHunks, [key]: hunks } })),
    toggleDiffFile: (key) =>
      upd((s) => {
        const next = new Set(s.expandedDiffFiles);
        if (next.has(key)) next.delete(key); else next.add(key);
        return { expandedDiffFiles: next };
      }),
    setAutoApprove: (v) => upd(() => ({ autoApprove: v })),
    setBlameOn: (v) => upd(() => ({ blameOn: v })),
    setBlame: (key, lines) => upd((s) => ({ blameByFile: { ...s.blameByFile, [key]: lines } })),
    setCommits: (c, hasMore) =>
      upd((st) => ({ commits: c, commitsHasMore: hasMore ?? c.length >= st.commitLimit })),
    loadMoreCommits: () =>
      upd((st) => (st.commitsHasMore ? { commitLimit: st.commitLimit + COMMIT_PAGE } : {})),
    setAnnotations: (a) => upd(() => ({ annotations: a })),
    addAnnotation: (a) =>
      upd((s) => (s.annotations.some((x) => x.id === a.id) ? {} : { annotations: [...s.annotations, a] })),
    resolveAnnotation: (aid) =>
      upd((s) => ({ annotations: s.annotations.map((a) => (a.id === aid ? { ...a, status: 'resolved' as const } : a)) })),
    removeAnnotation: (aid) => upd((s) => ({ annotations: s.annotations.filter((a) => a.id !== aid) })),
    setMrs: (mrs) => upd(() => ({ mrs })),
    upsertMr: (mr) =>
      upd((s) => ({
        mrs: s.mrs.some((m) => m.id === mr.id) ? s.mrs.map((m) => (m.id === mr.id ? mr : m)) : [...s.mrs, mr],
      })),
    setMrThreadsForRepo: (repoId, threads) =>
      upd((s) => ({ mrThreadsByRepo: { ...s.mrThreadsByRepo, [repoId]: threads } })),
    bumpMrs: () => upd((s) => ({ mrNonce: s.mrNonce + 1 })),
    setWorktreeStatus: (m) => upd(() => ({ worktreeStatus: m })),
    setRebaseConflict: (rc) => upd(() => ({ rebaseConflict: rc })),
    removePtySession: (pid) =>
      upd((s) => {
        const remaining = s.ptySessions.filter((p) => p.sessionId !== pid);
        return {
          ptySessions: remaining,
          activePtySessionId:
            s.activePtySessionId === pid ? (remaining[remaining.length - 1]?.sessionId ?? null) : s.activePtySessionId,
        };
      }),
  };
}

const boundActionsCache = new Map<string, SessionActions>();
function getBoundActions(id: string): SessionActions {
  let a = boundActionsCache.get(id);
  if (!a) {
    a = makeSessionActions(id);
    boundActionsCache.set(id, a);
  }
  return a;
}

/** A session's bound actions, for non-hook contexts (event handlers) that want to
 *  reuse the same reducers components call — instead of re-implementing patches. */
export function sessionActions(id: string): SessionActions {
  return getBoundActions(id);
}

// A frozen empty session so `useSession` reads stay safe when none is active.
const EMPTY_SESSION: SessionState = Object.freeze({
  id: '', kind: 'task', title: '', task: null, worktrees: [], repos: [], ...sessionDefaults(),
});

// Memoize the merged view per session record. `updateSessionState` produces a
// fresh SessionState object on every change, so keying by object identity means
// we rebuild the ~65-key view only when the session actually changed — unrelated
// store updates reuse the cached view. GC'd with the session (WeakMap).
const viewCache = new WeakMap<SessionState, SessionView>();

export function buildView(root: AppState, id: string | null): SessionView {
  const base = (id ? root.sessions[id] : undefined) ?? EMPTY_SESSION;
  const cached = viewCache.get(base);
  if (cached) return cached;
  // A real session's store key equals its id; EMPTY_SESSION carries id ''.
  const view: SessionView = {
    ...base,
    ...getBoundActions(base.id),
    activeTask: base.task,
    activeWorktrees: base.worktrees,
    activeRepos: base.repos,
  };
  viewCache.set(base, view);
  return view;
}
