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

/** A tab in the content pane: an open file, or a terminal bound to a PTY. */
export interface Tab {
  id: string;
  kind: 'file' | 'terminal';
  // file
  repoId?: string;
  path?: string;
  worktreePath?: string;
  content?: string | null; // null while loading
  dirty?: boolean;
  // terminal
  label?: string;
  ptySessionId?: string;
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
  tabs: Tab[];
  activeTabId: string | null;
}

export interface SessionsSlice {
  sessions: Record<string, SessionState>;
  openWorkspace: (p: WorkspacePayload) => void;
  closeSession: (id: string) => void;
  setActiveRepo: (sid: string, repoId: string) => void;
  setActiveWorktree: (sid: string, wtId: string) => void;
  setSidebar: (sid: string, panel: SidebarPanel) => void;
  openFileTab: (sid: string, repoId: string, path: string) => Promise<void>;
  openTerminalTab: (sid: string) => void;
  setActiveTab: (sid: string, tabId: string) => void;
  closeTab: (sid: string, tabId: string) => void;
  patchTab: (sid: string, tabId: string, patch: Partial<Tab>) => void;
}

/** The worktree the file tree/editor act on. */
export function activeWorktree(s: SessionState | undefined): Worktree | undefined {
  if (!s) return undefined;
  return s.worktrees.find((w) => w.id === s.activeWorktreeId);
}

const upd = (st: Store, sid: string, f: (s: SessionState) => SessionState) => {
  const s = st.sessions[sid];
  return s ? { sessions: { ...st.sessions, [sid]: f(s) } } : {};
};

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

  closeSession: (id) =>
    set((st) => {
      const sessions = { ...st.sessions };
      delete sessions[id];
      const rest = Object.keys(sessions);
      const stillActive = st.activeSessionId === id ? (rest[0] ?? null) : st.activeSessionId;
      return { sessions, activeSessionId: stillActive, view: stillActive ? st.view : 'home' };
    }),

  setActiveRepo: (sid, repoId) =>
    set((st) => upd(st, sid, (s) => ({
      ...s,
      activeRepoId: repoId,
      activeWorktreeId: s.worktrees.find((w) => w.repo_id === repoId)?.id ?? null,
    }))),

  setActiveWorktree: (sid, wtId) => set((st) => upd(st, sid, (s) => ({ ...s, activeWorktreeId: wtId }))),
  setSidebar: (sid, sidebar) => set((st) => upd(st, sid, (s) => ({ ...s, sidebar }))),

  openFileTab: async (sid, repoId, path) => {
    const tabId = `file::${repoId}::${path}`;
    const cur = get().sessions[sid];
    if (!cur) return;
    if (cur.tabs.some((t) => t.id === tabId)) {
      set((st) => upd(st, sid, (s) => ({ ...s, activeTabId: tabId })));
      return;
    }
    const wt = activeWorktree(cur);
    if (!wt) return;
    const tab: Tab = { id: tabId, kind: 'file', repoId, path, worktreePath: wt.path, content: null, dirty: false };
    set((st) => upd(st, sid, (s) => ({ ...s, tabs: [...s.tabs, tab], activeTabId: tabId })));
    try {
      const content = await call<string>('read_file', { worktreePath: wt.path, filePath: path });
      get().patchTab(sid, tabId, { content });
    } catch (e) {
      console.warn('read_file failed', e);
    }
  },

  openTerminalTab: (sid) =>
    set((st) => upd(st, sid, (s) => {
      const n = s.tabs.filter((t) => t.kind === 'terminal').length + 1;
      const id = `term::${n}::${Math.random().toString(36).slice(2, 7)}`;
      const tab: Tab = { id, kind: 'terminal', label: `terminal ${n}` };
      return { ...s, tabs: [...s.tabs, tab], activeTabId: id };
    })),

  setActiveTab: (sid, tabId) => set((st) => upd(st, sid, (s) => ({ ...s, activeTabId: tabId }))),

  closeTab: (sid, tabId) =>
    set((st) => upd(st, sid, (s) => {
      const tabs = s.tabs.filter((t) => t.id !== tabId);
      const activeTabId = s.activeTabId === tabId ? (tabs[tabs.length - 1]?.id ?? null) : s.activeTabId;
      return { ...s, tabs, activeTabId };
    })),

  patchTab: (sid, tabId, patch) =>
    set((st) => upd(st, sid, (s) => ({
      ...s,
      tabs: s.tabs.map((t) => (t.id === tabId ? { ...t, ...patch } : t)),
    }))),
});
