// Store contracts: the shapes every component and reducer agrees on.

import type {
  Task, Repo, Worktree, Mr, MrThread, Annotation, DiffResult, Hunk, CommitEntry,
  WorktreeStatus, BlameLine, AgentActivity, ConfirmationDto, Config, ThemeName,
  ReviewMr, HomeEntry, AgentSkill,
} from '../ipc/ipc';
import type { LayoutNode, SplitDir } from '../lib/layout';
import type { Chord } from '../lib/keys';
import type { CommandId, Keymap } from '../lib/keybindings';

/** Which header context picker is open, or none. */
export type PickerKind = 'session' | 'repo' | 'worktree' | null;

// ─── The app store's own contract ────────────────────────────────────────────

/** A content-search hit the editor marks: the needle and its 1-based line. */
export interface GrepHighlight {
  query: string;
  line: number;
}

export interface UiSlice {
  // ── Navigation ────────────────────────────────────────────────────────────
  view: AppView;
  setView: (v: AppView) => void;

  // ── Sidebar list focus ──────────────────────────────────────────────────────
  panelFocusNonce: number;
  requestPanelFocus: () => void;
  /** Bumped to focus the Source-control commit box. Also selects the git sidebar tab. */
  commitFocusNonce: number;
  requestCommitFocus: () => void;
  /** Bumped to focus the file-search input; `fileSearchMode` selects name or content search. */
  fileSearchFocusNonce: number;
  fileSearchMode: 'name' | 'text';
  requestFileSearchFocus: (mode?: 'name' | 'text') => void;

  // ── Grep match highlight ────────────────────────────────────────────────────
  // The match the Files panel's text-search cursor sits on. Null = off.
  grepHighlight: GrepHighlight | null;
  setGrepHighlight: (h: GrepHighlight | null) => void;

  // ── Terminal focus ──────────────────────────────────────────────────────────
  terminalFocusReq: number | null;
  requestTerminalFocus: () => void;
  /** The bottom terminal dock on Home. */
  terminalConsoleOpen: boolean;
  setTerminalConsoleOpen: (v: boolean) => void;

  // ── Command palette / overlays ─────────────────────────────────────────────
  commandPaletteOpen: boolean;
  setCommandPaletteOpen: (v: boolean) => void;
  /** Expands the file tree down to a directory. */
  revealDir: { path: string; nonce: number } | null;
  revealInTree: (path: string) => void;
  openPicker: PickerKind;
  setOpenPicker: (p: PickerKind) => void;
  /** Highlighted row in the open picker. */
  pickerCursor: number;
  setPickerCursor: (n: number) => void;
  addRepoOpen: boolean;
  addWorktreeOpen: boolean;
  setAddRepoOpen: (v: boolean) => void;
  setAddWorktreeOpen: (v: boolean) => void;

  // ── Settings ──────────────────────────────────────────────────────────────
  /** The view closing settings returns to. */
  settingsReturnTo: AppView;
  openSettings: () => void;
  closeSettings: () => void;

  // ── Editor: Vim mode (persisted to localStorage) ───────────────────────────
  vimMode: boolean;
  setVimMode: (v: boolean) => void;
}

export interface HomeSlice {
  // ── Review queue (Home + rail badge) ────────────────────────────────────────
  /** Open MRs where the user is a reviewer. Null until the first fetch lands. */
  reviewQueue: ReviewMr[] | null;
  /** Refetches the queue; a failure only warns. */
  refreshReviewQueue: () => Promise<void>;

  // ── Home snapshot (local state of every live session) ───────────────────────
  /** Per-session repo/worktree/MR state. Null until the first fetch lands. */
  homeSnapshot: HomeEntry[] | null;
  /** True while a refresh is in flight (drives the refresh spinner). */
  homeLoading: boolean;
  /** Refetches. `forceMr` also bypasses the cached CI/thread counts. */
  refreshHome: (forceMr?: boolean) => Promise<void>;

  // ── Task list ─────────────────────────────────────────────────────────────
  tasks: Task[];
  setTasks: (tasks: Task[]) => void;
  /** Re-read the queue from every configured source. */
  refreshTasks: () => Promise<void>;
  upsertTask: (task: Task) => void;
}

export interface SessionsSlice {
  sessions: Record<string, SessionState>;
  sessionOrder: string[];
  activeSessionId: string | null;
  /** Opens or focuses a session and makes it active. `focus: false` skips navigation. */
  openSession: (input: { kind: SessionKind; task?: Task | null; worktrees?: Worktree[]; repos?: Repo[]; focus?: boolean }) => string;
  focusSession: (id: string) => void;
  /** Removes a session from the store. Stop its PTYs first via endSession. */
  closeSession: (id: string) => void;
  /** Patches one session's state with an object patch or a recipe. */
  updateSession: (id: string, patch: Partial<SessionState> | ((s: SessionState) => Partial<SessionState>)) => void;
  /** Recomputes a session's worktree git status. */
  refreshStatusFor: (id: string) => Promise<void>;
  /** Bumps diffNonce and clears cached hunks. The current diff stays visible until the refetch lands. */
  invalidateDiff: (id: string) => void;
  /** Forces an MR + threads reload for a session. */
  invalidateMrs: (id: string) => void;
}

export interface ConfirmationsSlice {
  pendingConfirmations: ConfirmationDto[];
  addConfirmation: (c: ConfirmationDto) => void;
  removeConfirmation: (id: string) => void;
  /** True while the approvals modal is deferred. A new confirmation un-defers it. */
  confirmationsMinimized: boolean;
  setConfirmationsMinimized: (v: boolean) => void;
}

export interface AgentSlice {
  /** Agent activity keyed by task short id, reported by Claude Code hooks. */
  agentActivity: Record<string, AgentActivity>;
  setAgentActivity: (a: AgentActivity) => void;
  dropAgentActivity: (taskId: string) => void;
  hydrateAgentActivity: () => Promise<void>;
  /** The agent console is expanded. It addresses the focused session. */
  consoleOpen: boolean;
  setConsoleOpen: (v: boolean) => void;
  /** Bumped by the keybinding to pull DOM focus into the console's terminal. */
  consoleFocusNonce: number;
  requestConsoleFocus: () => void;
  /** The agent column fills the body. */
  agentMaximized: boolean;
  setAgentMaximized: (v: boolean) => void;
  /** The agent lives in its own OS window. Persisted. */
  agentDetached: boolean;
  setAgentDetached: (v: boolean) => void;
  /** The running-agents list shows as a column of the agent panel. Persisted. */
  agentsSidebarOpen: boolean;
  setAgentsSidebarOpen: (v: boolean) => void;
  /** Its width. Persisted. */
  agentsSidebarWidth: number;
  setAgentsSidebarWidth: (w: number) => void;
}

export interface KeybindingsSlice {
  // Defaults + user overrides, persisted to localStorage.
  keymap: Keymap;
  setBinding: (id: CommandId, chords: Chord[]) => void;
  /** Put one command back on this platform's default. */
  resetBinding: (id: CommandId) => void;
  resetKeymap: () => void;
  /** True while the Settings rebind UI captures a keystroke; the global keymap is suspended. */
  capturingKey: boolean;
  setCapturingKey: (v: boolean) => void;
}

export interface ConfigSlice {
  config: Config | null;
  setConfig: (c: Config | null) => void;
  setTheme: (theme: ThemeName) => void;
  setFontSize: (px: number) => void;
  setFontFamily: (family: string) => void;
  setAgentFontFamily: (family: string) => void;
  setSuggestActions: (v: boolean) => void;

  // ── Status ────────────────────────────────────────────────────────────────
  syncStatus: 'idle' | 'syncing' | 'error';
  setSyncStatus: (s: 'idle' | 'syncing' | 'error') => void;
  lastError: string | null;
  setLastError: (e: string | null) => void;
}

export interface SkillsSlice {
  /** What the agent can be asked to do, core first. */
  skills: AgentSkill[];
  loadSkills: () => Promise<void>;
  /** A skill file changed since the agents started. */
  skillsStale: boolean;
  setSkillsStale: (stale: boolean) => void;
}

export interface NotificationsSlice {
  /** Newest first, capped. Two views: transient toasts + the notification centre. */
  notifications: AppNotification[];
  /** Ids currently showing as toasts. Dismissing a toast keeps the feed entry. */
  toastIds: string[];
  notify: (n: NotificationInput) => void;
  dismissToast: (id: string) => void;
  notificationsOpen: boolean;
  setNotificationsOpen: (v: boolean) => void;
  /** Marks one notification as seen. */
  markNotificationRead: (id: string) => void;
  markNotificationsRead: () => void;
  clearNotifications: () => void;
}

/** The composed store: every slice, one state. */
export type AppState = UiSlice & HomeSlice & SessionsSlice & ConfirmationsSlice &
  AgentSlice & KeybindingsSlice & ConfigSlice & NotificationsSlice & SkillsSlice;

export type AppView = 'home' | 'workspace' | 'settings';
export type SidebarTab = 'files' | 'git' | 'annotations';

// ─── Notifications ────────────────────────────────────────────────────────────
// One feed; toasts and the notification centre are two views of it.

export type NotificationKind = 'success' | 'error' | 'attention' | 'info';

/** Who produced it; drives the icon. */
export type NotificationSource = 'agent' | 'mcp' | 'git' | 'mr' | 'task' | 'files' | 'app';

export interface NotificationInput {
  kind: NotificationKind;
  /** One line. */
  title: string;
  /** The specifics: an error message, the tool being asked about, a file list. */
  detail?: string;
  source?: NotificationSource;
  /** Task short id this concerns — rendered as a chip, and used for grouping. */
  taskId?: string;
  /** Repo/project name, when the event belongs to one repo of a session. */
  repo?: string;
  /** Where clicking should take the user. */
  goTo?: { taskId?: string; agent?: boolean };
}

export interface AppNotification extends NotificationInput {
  id: string;
  at: number;
  read: boolean;
  /** Repeats of the same event collapse into one row with a count. */
  count: number;
  /** Shown once as a toast; never in the feed or the badge. */
  ephemeral?: boolean;
}
/** Sub-modes of the Source-control panel. */
export type GitSubTab = 'changes' | 'commits' | 'forge';
/** Diff comparison base: vs the default branch, vs this branch's remote, or uncommitted work. */
export type DiffMode = 'vs-main' | 'vs-remote' | 'working';

/** A workspace tab shows one file, viewed either as a diff or an editable buffer. */
export type TabView = 'diff' | 'edit';
export interface EditorTab {
  id: string;        // `${repoId}::${filePath}` — unique within a pane
  repoId: string;
  filePath: string;
  view: TabView;
  /** 'file' = one file (default); 'changes' = the repo's "All changes" review;
   *  'commit' = one commit's diff; 'terminal' = a shell. */
  kind?: 'file' | 'changes' | 'commit' | 'terminal';
  /** Commit sha for kind='commit'. */
  sha?: string;
  /** Bound PTY session for kind='terminal' (set once the session starts). */
  ptySessionId?: string;
  /** Tab-strip label for non-file kinds (short sha, "!42"/"#42"). */
  label?: string;
  /** Seed cursor line for an edit view (e.g. jumping from a grep result). */
  cursorLine?: number;
  /** Transient preview tab, at most one per pane. Enter commits it, Esc discards it. */
  preview?: boolean;
}
/** A pane is one tab group; panes arrange in a recursive split tree (`layout`). */
export interface WorkspacePane {
  id: string;
  tabs: EditorTab[];
  activeTabId: string | null;
}

export interface PtySessionState {
  sessionId: string;
  taskId: string;
  ptyType: 'agent' | 'terminal';
  label: string;
}

// ─── Session layer ────────────────────────────────────────────────────────────
// A session is one open workspace: a task, an explorer, or an MR review.
// Each holds its own tabs, diff, annotations and agent terminals.

import type { SessionKind } from '../ipc/ipc';
export type { SessionKind };

export type WorkspaceMode = 'overview' | 'code';

export interface SessionState {
  id: string;            // session id (distinct from the task short_id)
  kind: SessionKind;
  title: string;         // tab label
  /** The Overview page or the code panes. Opening any tab flips to code. */
  workspaceMode: WorkspaceMode;

  // ── task-kind payload ──
  task: Task | null;
  worktrees: Worktree[];
  repos: Repo[];
  activeRepoId: string | null;
  /** The worktree git ops target. `activeRepoId` is derived from it. */
  activeWorktreeId: string | null;

  panes: WorkspacePane[];
  activePaneId: string;
  /** Recursive split arrangement over pane ids (leaf per pane). */
  layout: LayoutNode;
  /** When set, only this pane renders (others stay mounted, hidden). */
  maximizedPaneId: string | null;
  /** Bumped when a non-preview tab is opened or committed. Preview opens never bump it. */
  editorFocusNonce: number;
  sidebarTab: SidebarTab;
  /** The panel column is hidden. */
  sidebarCollapsed: boolean;
  gitSubTab: GitSubTab;

  diff: DiffResult | null;
  diffHunks: Record<string, Hunk[]>;
  diffMode: DiffMode;
  diffNonce: number;
  /** Files expanded in the "All changes" review, keyed `${repoId}/${path}`. */
  expandedDiffFiles: Set<string>;

  /** Approve this session's agent write ops without asking. In memory only; never written to config. */
  autoApprove: boolean;

  /** Show the per-line author gutter in the editor and diff views. */
  blameOn: boolean;
  /** Blame per file, keyed `${repoId}/${path}`. Cleared with the diff cache. */
  blameByFile: Record<string, BlameLine[]>;

  worktreeStatus: Record<string, WorktreeStatus>;
  commits: CommitEntry[];
  /** How many commits the log asks for. Grows as the list is scrolled. */
  commitLimit: number;
  /** False once a fetch returns fewer commits than it asked for. */
  commitsHasMore: boolean;
  annotations: Annotation[];
  mrs: Mr[];
  mrThreadsByRepo: Record<string, MrThread[]>;
  /** Bumped to refetch MRs + their threads (after mr.* ops, on Forge tab open). */
  mrNonce: number;
  /** Active rebase conflict for one of this session's worktrees, or null. */
  rebaseConflict: { worktreeId: string; files: string[] } | null;

  ptySessions: PtySessionState[];
  activePtySessionId: string | null;
}

/** The per-session actions a component reaches through `useSession`. */
export interface SessionActions {
  setWorkspaceMode: (m: WorkspaceMode) => void;
  setDiffMode: (m: DiffMode) => void;
  bumpDiff: () => void;
  refreshStatus: () => Promise<void>;
  /** Split the active pane (row = right, col = below); new pane takes focus. */
  splitPane: (dir: SplitDir) => void;
  /** Dock-style split: wraps the whole layout so the new pane spans full height (row) or width (col). */
  splitRootPane: (dir: SplitDir, ratio?: number) => void;
  /** Close a pane; its tabs merge into the surviving sibling. */
  closePane: (paneId: string) => void;
  setSplitRatio: (splitId: string, ratio: number) => void;
  /** Maximize/restore the active pane (others stay mounted, hidden). */
  toggleMaximizePane: () => void;
  /** Cycle focus through panes in visual order. */
  focusNextPane: () => void;
  openTab: (input: OpenTabInput, opts?: { paneId?: string }) => void;
  /** Clear the preview flag on the pane's preview tab (Enter — keep it open). */
  commitPreview: (paneId: string) => void;
  /** Remove the pane's preview tab (Esc — cancel). */
  discardPreview: (paneId: string) => void;
  closeTab: (paneId: string, tabId: string) => void;
  setActiveTab: (paneId: string, tabId: string) => void;
  setTabView: (paneId: string, tabId: string, view: TabView) => void;
  /** Bind a terminal tab to its started PTY session. */
  setTabPty: (paneId: string, tabId: string, ptySessionId: string) => void;
  focusPane: (paneId: string) => void;
  /** Selects a repo and its first worktree, or keeps the current worktree when it belongs to the repo. */
  setActiveRepoId: (id: string | null) => void;
  /** Select the exact worktree (multi-worktree repos); syncs activeRepoId. */
  setActiveWorktreeId: (id: string | null) => void;
  setSidebarTab: (t: SidebarTab) => void;
  setSidebarCollapsed: (v: boolean) => void;
  setGitSubTab: (t: GitSubTab) => void;
  setDiff: (d: DiffResult | null) => void;
  setDiffHunks: (key: string, hunks: Hunk[]) => void;
  /** Toggle a file's expanded state in the "All changes" review. */
  toggleDiffFile: (key: string) => void;
  setAutoApprove: (v: boolean) => void;
  setBlameOn: (v: boolean) => void;
  setBlame: (key: string, lines: BlameLine[]) => void;
  /** Replace the log. `hasMore` is false when git returned fewer than requested. */
  setCommits: (c: CommitEntry[], hasMore?: boolean) => void;
  /** Ask for another page of commits (the fetch effect reloads). */
  loadMoreCommits: () => void;
  setAnnotations: (a: Annotation[]) => void;
  /** Adds one; a repeat id is ignored. */
  addAnnotation: (a: Annotation) => void;
  /** Replace one annotation's body (after `update_annotation` lands). */
  updateAnnotation: (id: string, content: string) => void;
  resolveAnnotation: (id: string) => void;
  /** Drop an annotation entirely (after `delete_annotation` lands). */
  removeAnnotation: (id: string) => void;
  setMrs: (mrs: Mr[]) => void;
  upsertMr: (mr: Mr) => void;
  setMrThreadsForRepo: (repoId: string, threads: MrThread[]) => void;
  /** Refetch MRs + threads (bumps mrNonce; useWorkspaceData reloads). */
  bumpMrs: () => void;
  setWorktreeStatus: (m: Record<string, WorktreeStatus>) => void;
  setRebaseConflict: (rc: SessionState['rebaseConflict']) => void;
  removePtySession: (id: string) => void;
}

/** `active*` aliases over the session's task, worktrees and repos. */
export interface SessionAliases {
  activeTask: Task | null;
  activeWorktrees: Worktree[];
  activeRepos: Repo[];
}

/** What `useSession`'s selector sees: a session merged with its bound actions. */
export type SessionView = SessionState & SessionActions & SessionAliases;

export interface OpenTabInput {
  repoId: string;
  filePath: string;
  view: TabView;
  kind?: EditorTab['kind'];
  sha?: string;
  ptySessionId?: string;
  label?: string;
  cursorLine?: number;
  preview?: boolean;
}
