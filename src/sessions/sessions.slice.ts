import type { StateCreator } from 'zustand';
import { call } from '../shared/ipc/client';
import type { Store } from '../shared/store/types';
import type { TaskView, Worktree, Repo, SessionKind } from '../shared/ipc/generated';

/** What `workspace_ready` carries — see task_manager (open_* commands). */
export interface WorkspacePayload {
  task: TaskView;
  worktrees: Worktree[];
  repos: Repo[];
  kind: SessionKind;
}

export type SidebarPanel = 'files' | 'git' | 'notes';

export interface FileTab {
  id: string; // `${repoId}::${path}`
  repoId: string;
  path: string;
  content: string | null; // null while loading
}

export interface SessionState {
  id: string;
  kind: SessionKind;
  title: string;
  worktrees: Worktree[];
  repos: Repo[];
  activeRepoId: string | null;
  activeWorktreeId: string | null;
  sidebar: SidebarPanel;
  tabs: FileTab[];
  activeTabId: string | null;
}

export interface SessionsSlice {
  sessions: Record<string, SessionState>;
  /** Ingest a `workspace_ready` payload into a session (keyed by task short_id). */
  openWorkspace: (p: WorkspacePayload) => void;
  closeSession: (id: string) => void;
  setActiveRepo: (sid: string, repoId: string) => void;
  setActiveWorktree: (sid: string, wtId: string) => void;
  setSidebar: (sid: string, panel: SidebarPanel) => void;
  openFileTab: (sid: string, repoId: string, path: string) => Promise<void>;
  setActiveTab: (sid: string, tabId: string) => void;
  closeTab: (sid: string, tabId: string) => void;
}

/** The worktree the file tree/editor act on. */
export function activeWorktree(s: SessionState | undefined): Worktree | undefined {
  if (!s) return undefined;
  return s.worktrees.find((w) => w.id === s.activeWorktreeId);
}

export const sessionsSlice: StateCreator<Store, [], [], SessionsSlice> = (set, get) => ({
  sessions: {},

  openWorkspace: (p) => {
    const id = p.task.short_id;
    const firstRepo = p.repos[0]?.id ?? null;
    const firstWt = p.worktrees.find((w) => w.repo_id === firstRepo)?.id ?? p.worktrees[0]?.id ?? null;
    set((st) => {
      const existing = st.sessions[id];
      const next: SessionState = {
        id,
        kind: p.kind,
        title: p.task.title || id,
        worktrees: p.worktrees,
        repos: p.repos,
        activeRepoId: existing?.activeRepoId ?? firstRepo,
        activeWorktreeId: existing?.activeWorktreeId ?? firstWt,
        sidebar: existing?.sidebar ?? 'files',
        tabs: existing?.tabs ?? [],
        activeTabId: existing?.activeTabId ?? null,
      };
      return { sessions: { ...st.sessions, [id]: next }, activeSessionId: id };
    });
  },

  closeSession: (id) => {
    set((st) => {
      const sessions = { ...st.sessions };
      delete sessions[id];
      const rest = Object.keys(sessions);
      const stillActive = st.activeSessionId === id ? (rest[0] ?? null) : st.activeSessionId;
      return {
        sessions,
        activeSessionId: stillActive,
        view: stillActive ? st.view : 'home',
      };
    });
  },

  setActiveRepo: (sid, repoId) =>
    set((st) => {
      const s = st.sessions[sid];
      if (!s) return {};
      const wt = s.worktrees.find((w) => w.repo_id === repoId)?.id ?? null;
      return { sessions: { ...st.sessions, [sid]: { ...s, activeRepoId: repoId, activeWorktreeId: wt } } };
    }),

  setActiveWorktree: (sid, wtId) =>
    set((st) => {
      const s = st.sessions[sid];
      return s ? { sessions: { ...st.sessions, [sid]: { ...s, activeWorktreeId: wtId } } } : {};
    }),

  setSidebar: (sid, sidebar) =>
    set((st) => {
      const s = st.sessions[sid];
      return s ? { sessions: { ...st.sessions, [sid]: { ...s, sidebar } } } : {};
    }),

  openFileTab: async (sid, repoId, path) => {
    const tabId = `${repoId}::${path}`;
    const cur = get().sessions[sid];
    if (!cur) return;
    if (cur.tabs.some((t) => t.id === tabId)) {
      set((st) => ({ sessions: { ...st.sessions, [sid]: { ...st.sessions[sid], activeTabId: tabId } } }));
      return;
    }
    const wt = activeWorktree(cur);
    if (!wt) return;
    // add the tab immediately (content loading), then fill it
    set((st) => {
      const s = st.sessions[sid];
      const tab: FileTab = { id: tabId, repoId, path, content: null };
      return { sessions: { ...st.sessions, [sid]: { ...s, tabs: [...s.tabs, tab], activeTabId: tabId } } };
    });
    try {
      const content = await call<string>('read_file', { worktreePath: wt.path, filePath: path });
      set((st) => {
        const s = st.sessions[sid];
        if (!s) return {};
        return {
          sessions: {
            ...st.sessions,
            [sid]: { ...s, tabs: s.tabs.map((t) => (t.id === tabId ? { ...t, content } : t)) },
          },
        };
      });
    } catch (e) {
      console.warn('read_file failed', e);
    }
  },

  setActiveTab: (sid, tabId) =>
    set((st) => {
      const s = st.sessions[sid];
      return s ? { sessions: { ...st.sessions, [sid]: { ...s, activeTabId: tabId } } } : {};
    }),

  closeTab: (sid, tabId) =>
    set((st) => {
      const s = st.sessions[sid];
      if (!s) return {};
      const tabs = s.tabs.filter((t) => t.id !== tabId);
      const activeTabId = s.activeTabId === tabId ? (tabs[tabs.length - 1]?.id ?? null) : s.activeTabId;
      return { sessions: { ...st.sessions, [sid]: { ...s, tabs, activeTabId } } };
    }),
});
