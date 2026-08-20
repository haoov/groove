import { create } from 'zustand';
import { createContext, useContext } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { notificationSlice } from './notifications';
import type { AgentActivity, HomeEntry, ReviewMr, WorktreeStatus } from '../ipc/ipc';
import { applyTheme, applyFontSize, applyFontFamily } from '../lib/theme';
import {
  splitRoot, leafOrder, setRatio as setLayoutRatio,
  type LayoutNode, type SplitDir,
} from '../lib/layout';
import { assignBinding, defaultKeymap, loadKeymap, saveKeymap, clearKeymap } from '../lib/keybindings';
import type { AppState, SessionActions, SessionState, SessionView } from './types';
import {
  COMMIT_PAGE, bumpDiffRecipe, closePaneReducer, closeTabReducer, commitPreviewReducer,
  discardPreviewReducer, newPaneId, newWorkspaceSession, openTabReducer, sessionDefaults,
  sessionTitle, splitPaneReducer,
} from './session';

export type { LayoutNode, SplitDir };
// A pane predicate components need (the terminal dock is styled and placed by it).
export { isTerminalPane } from './session';
// The store stays the single import site for its own contracts: components keep
// importing types from './' instead of reaching into ./types.
export type * from './types';

export const useStore = create<AppState>((set, get) => ({
  // Navigation
  view: 'home',
  setView: (v) =>
    // Leaving the workspace drops a maximized agent: the rule that gives it the
    // pane area has no pane area to take on Home.
    set(v === 'workspace' ? { view: v } : { view: v, agentMaximized: false }),

  // Review queue
  reviewQueue: null,
  refreshReviewQueue: async () => {
    try {
      const queue = await invoke<ReviewMr[]>('list_review_mrs');
      set({ reviewQueue: queue });
    } catch (e) {
      // Used to be a console warning only, which meant an empty review list was
      // indistinguishable from a broken `glab`.
      get().notify({
        kind: 'error',
        source: 'mr',
        title: 'Could not load the review queue',
        detail: String(e),
      });
    }
  },

  // Home snapshot
  homeSnapshot: null,
  homeLoading: false,
  refreshHome: async (forceMr = false) => {
    set({ homeLoading: true });
    try {
      set({ homeSnapshot: await invoke<HomeEntry[]>('get_home_snapshot', { forceMr }) });
    } catch (e) {
      get().notify({
        kind: 'error',
        source: 'app',
        title: 'Could not refresh Home',
        detail: String(e),
      });
    } finally {
      set({ homeLoading: false });
    }
  },

  // Sessions
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
        // Only a session with a WORKSPACE can be the successor. The desk is
        // always in sessionOrder and never closes, so picking it left the app in
        // 'workspace' view with nothing to render — a blank page after finishing
        // the last real session.
        const workspaces = s.sessionOrder.filter((x) => s.sessions[x]?.kind !== 'desk');
        const at = workspaces.indexOf(id);
        const remaining = workspaces.filter((x) => x !== id);
        activeSessionId = remaining[at] ?? remaining[at - 1] ?? null;
        if (!activeSessionId) view = 'home';
      }
      return { sessions, sessionOrder, activeSessionId, view };
    }),
  updateSession: (id, patch) =>
    updateSessionState(id, typeof patch === 'function' ? patch : () => patch),
  refreshStatusFor: (id) => doRefreshStatus(id),
  invalidateDiff: (id) => updateSessionState(id, bumpDiffRecipe),
  invalidateMrs: (id) => updateSessionState(id, (s) => ({ mrNonce: s.mrNonce + 1 })),

  // Task list
  tasks: [],
  setTasks: (tasks) => set({ tasks }),
  upsertTask: (task) =>
    set((s) => {
      const tasks = s.tasks.some((t) => t.short_id === task.short_id)
        ? s.tasks.map((t) => (t.short_id === task.short_id ? task : t))
        : [...s.tasks, task];
      // Keep any open session showing this task in sync (task object + tab label).
      let sessions = s.sessions;
      for (const id of s.sessionOrder) {
        const sess = s.sessions[id];
        if (sess?.task?.short_id === task.short_id) {
          if (sessions === s.sessions) sessions = { ...s.sessions };
          sessions[id] = { ...sess, task, title: sessionTitle(sess.kind, task) };
        }
      }
      return { tasks, sessions };
    }),

  // Confirmations
  pendingConfirmations: [],
  addConfirmation: (c) =>
    set((s) => ({
      pendingConfirmations: s.pendingConfirmations.some((p) => p.id === c.id)
        ? s.pendingConfirmations
        : [...s.pendingConfirmations, c],
      // A new request always surfaces the modal, even if the user deferred earlier.
      confirmationsMinimized: false,
    })),
  removeConfirmation: (id) =>
    set((s) => ({
      pendingConfirmations: s.pendingConfirmations.filter((c) => c.id !== id),
    })),
  confirmationsMinimized: false,
  setConfirmationsMinimized: (v) => set({ confirmationsMinimized: v }),

  // Sidebar list focus
  panelFocusNonce: 0,
  requestPanelFocus: () => set((s) => ({ panelFocusNonce: s.panelFocusNonce + 1 })),
  commitFocusNonce: 0,
  requestCommitFocus: () =>
    set((s) => {
      const id = s.activeSessionId;
      const sess = id ? s.sessions[id] : null;
      return {
        commitFocusNonce: s.commitFocusNonce + 1,
        // Also un-collapse: the commit box cannot take focus while the column it
        // lives in is hidden.
        ...(sess && id
          ? {
              sessions: {
                ...s.sessions,
                [id]: { ...sess, sidebarTab: 'git' as const, sidebarCollapsed: false },
              },
            }
          : {}),
      };
    }),
  fileSearchFocusNonce: 0,
  fileSearchMode: 'name',
  requestFileSearchFocus: (mode = 'name') =>
    set((s) => ({ fileSearchMode: mode, fileSearchFocusNonce: s.fileSearchFocusNonce + 1 })),

  // Grep match highlight
  grepHighlight: null,
  // Under two characters is not a search worth painting — it would mark half the file.
  setGrepHighlight: (h) => set({ grepHighlight: h && h.query.length >= 2 ? h : null }),

  // Terminal focus
  terminalFocusReq: null,
  requestTerminalFocus: () => set((s) => ({ terminalFocusReq: (s.terminalFocusReq ?? 0) + 1 })),
  terminalConsoleOpen: false,
  setTerminalConsoleOpen: (v) => set({ terminalConsoleOpen: v }),

  // Agent activity + console
  agentActivity: {},
  setAgentActivity: (a) =>
    set((s) => ({ agentActivity: { ...s.agentActivity, [a.task_id]: a } })),
  dropAgentActivity: (taskId) =>
    set((s) => {
      if (!(taskId in s.agentActivity)) return {};
      const next = { ...s.agentActivity };
      delete next[taskId];
      return { agentActivity: next };
    }),
  hydrateAgentActivity: async () => {
    try {
      const rows = await invoke<AgentActivity[]>('get_agent_activity');
      set({ agentActivity: Object.fromEntries(rows.map((r) => [r.task_id, r])) });
    } catch {
      // Best-effort: an empty map just means "unknown", which is the honest state.
    }
  },
  consoleOpen: false,
  setConsoleOpen: (v) =>
    // Closing drops the maximize too: reopening straight into a full-screen agent
    // is not what the last Alt+A meant.
    set(v ? { consoleOpen: true } : { consoleOpen: false, agentMaximized: false }),
  consoleFocusNonce: 0,
  requestConsoleFocus: () =>
    set((s) => ({ consoleOpen: true, consoleFocusNonce: s.consoleFocusNonce + 1 })),
  agentMaximized: false,
  setAgentMaximized: (v) => set({ agentMaximized: v }),

  // Command palette
  revealDir: null,
  revealInTree: (path) =>
    set((s) => ({ revealDir: { path, nonce: (s.revealDir?.nonce ?? 0) + 1 } })),
  dockOpen: localStorage.getItem('wb.dockOpen') !== '0',
  setDockOpen: (v) => {
    localStorage.setItem('wb.dockOpen', v ? '1' : '0');
    set({ dockOpen: v });
  },
  dockFocusNonce: 0,
  requestDockFocus: () =>
    set((s) => {
      localStorage.setItem('wb.dockOpen', '1');
      return { dockOpen: true, dockFocusNonce: s.dockFocusNonce + 1 };
    }),
  addRepoOpen: false,
  setAddRepoOpen: (v) => set({ addRepoOpen: v }),
  repoSwitcherOpen: false,
  setRepoSwitcherOpen: (v) => set({ repoSwitcherOpen: v }),
  commandPaletteOpen: false,
  setCommandPaletteOpen: (v) => set({ commandPaletteOpen: v }),

  settingsOpen: false,
  setSettingsOpen: (v) => set({ settingsOpen: v }),

  // Vim mode — defaults on (the editor + readonly-diff use vim navigation).
  vimMode: (() => {
    try {
      const v = localStorage.getItem('workbench.vimMode');
      return v === null ? true : v === 'true';
    } catch {
      return true;
    }
  })(),
  setVimMode: (v) => {
    try { localStorage.setItem('workbench.vimMode', String(v)); } catch { /* ignore */ }
    set({ vimMode: v });
  },

  // Keybindings
  keymap: loadKeymap(),
  setBinding: (id, chords) =>
    set((s) => {
      // Exclusive: the chord is taken off whatever held it. Otherwise two
      // commands share it and declaration order silently decides the winner.
      const keymap = assignBinding(s.keymap, id, chords);
      saveKeymap(keymap);
      return { keymap };
    }),
  resetKeymap: () => {
    clearKeymap();
    set({ keymap: defaultKeymap() });
  },
  capturingKey: false,
  setCapturingKey: (v) => set({ capturingKey: v }),

  // Task wizard
  wizardTask: null,
  setWizardTask: (t) => set({ wizardTask: t }),

  // Config
  config: null,
  setConfig: (c) => set({ config: c }),
  setTheme: (theme) => {
    applyTheme(theme);
    invoke('set_theme', { theme }).catch((e) => set({ lastError: String(e) }));
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, theme } } } : {}));
  },
  setFontSize: (px) => {
    applyFontSize(px);
    invoke('set_font_size', { fontSize: px }).catch((e) => set({ lastError: String(e) }));
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, font_size: px } } } : {}));
  },
  setFontFamily: (family) => {
    applyFontFamily(family);
    invoke('set_font_family', { fontFamily: family }).catch((e) => set({ lastError: String(e) }));
    // The store update is what tells terminalHost's subscriber to re-skin xterm,
    // which reads a JS string rather than the CSS variable.
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, font_family: family } } } : {}));
  },

  // Status
  syncStatus: 'idle',
  setSyncStatus: (s) => set({ syncStatus: s }),
  lastError: null,
  setLastError: (e) => {
    set({ lastError: e });
    // Every error also lands in the feed, so a later one can't erase it. Call
    // sites that know more (which task, which subsystem) should `notify` directly.
    if (e) get().notify({ kind: 'error', title: e });
  },

  ...notificationSlice(set),
}));

// ─── Session state mutation + bound actions ─────────────────────────────────────

/** Immutably patch one session, merging only the changed fields. */
function updateSessionState(id: string, recipe: (s: SessionState) => Partial<SessionState>) {
  if (!id) return;
  useStore.setState((st) => {
    const sess = st.sessions[id];
    if (!sess) return {};
    return { sessions: { ...st.sessions, [id]: { ...sess, ...recipe(sess) } } };
  });
}

/** Recompute git status for a session's active worktrees. Shared by the bound
 *  per-session action and the root `refreshStatusFor` (used by event handlers). */
async function doRefreshStatus(id: string) {
  const sess = useStore.getState().sessions[id];
  if (!sess) return;
  const entries = await Promise.allSettled(
    sess.worktrees.map((w) => invoke<WorktreeStatus>('get_worktree_status', { worktreeId: w.id }))
  );
  const next: Record<string, WorktreeStatus> = {};
  for (const r of entries) {
    if (r.status === 'fulfilled') next[r.value.worktree_id] = r.value;
  }
  updateSessionState(id, () => ({ worktreeStatus: next }));
}

/** Per-session actions, bound to a session id. Cached so references stay stable. */
function makeSessionActions(id: string): SessionActions {
  const upd = (recipe: (s: SessionState) => Partial<SessionState>) => updateSessionState(id, recipe);
  return {
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
        // A real open focuses the editor; a preview open leaves focus in the search input.
        return input.preview ? patch : { ...patch, editorFocusNonce: s.editorFocusNonce + 1 };
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
    setActiveRepoId: (rid) => upd(() => ({ activeRepoId: rid })),
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

function buildView(root: AppState, id: string | null): SessionView {
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

// ─── Session context + hooks ────────────────────────────────────────────────────

/** Provided by SessionWorkspaces so descendants read *their* session, even when
 *  hidden. When absent (app-level chrome), hooks fall back to the active session. */
export const SessionIdContext = createContext<string | null>(null);

/**
 * Read from (or act on) the contextual session. The selector receives the
 * session's fields merged with its bound actions. Always select a single value
 * (e.g. `useSession((s) => s.diff)`), never the whole view, to keep subscriptions
 * fine-grained and avoid render loops.
 */
export function useSession<T>(selector: (s: SessionView) => T): T {
  const ctx = useContext(SessionIdContext);
  return useStore((root) => selector(buildView(root, ctx ?? root.activeSessionId)));
}

// ─── Non-hook accessors (for event handlers using getState) ──────────────────────

export function getSession(state: AppState, id: string | null): SessionState | null {
  return id ? state.sessions[id] ?? null : null;
}
export function getActiveSession(state: AppState): SessionState | null {
  return getSession(state, state.activeSessionId);
}
export function findSessionByTask(state: AppState, shortId: string): SessionState | null {
  const id = state.sessionOrder.find((sid) => state.sessions[sid]?.task?.short_id === shortId);
  return id ? state.sessions[id] : null;
}
export function findSessionByPty(state: AppState, ptySessionId: string): SessionState | null {
  const id = state.sessionOrder.find((sid) =>
    state.sessions[sid]?.ptySessions.some((p) => p.sessionId === ptySessionId)
  );
  return id ? state.sessions[id] : null;
}
export function findSessionByWorktree(state: AppState, worktreeId: string): SessionState | null {
  const id = state.sessionOrder.find((sid) =>
    state.sessions[sid]?.worktrees.some((w) => w.id === worktreeId)
  );
  return id ? state.sessions[id] : null;
}
