// ── The command registry ──────────────────────────────────────────────────────
// One CommandSpec per CommandId. Labels, groups and chords live in shared/lib/pure/keybindings.ts.

import {
  ArrowRightLeft, Bell, Bot, ChevronLeft, ChevronRight, Columns2, Command, Compass,
  FileSearch, Fingerprint, FolderGit2, FolderPlus, FolderTree, GitBranch, GitCommitVertical,
  Keyboard, Layers, LayoutGrid, Maximize2, PanelRight, Repeat, RotateCcw, Rows2, Search,
  Settings, SquarePen, SquareTerminal, StickyNote, X, ZoomIn, ZoomOut, type LucideIcon,
} from 'lucide-react';
import {
  useStore, sessionActions,
  type AppState, type GitSubTab, type SessionState, type SidebarTab,
} from '../../store';
import { toggleTerminal } from './panes';
import { toggleAgentsSidebar } from './agentsSidebar';
import { DEFAULT_FONT_SIZE, FONT_MIN, FONT_MAX } from '../../ipc/ipc';
import { COMMANDS, shortcutLabel, type CommandId, type Keymap } from '../pure/keybindings';

export interface CommandContext {
  st: AppState;
  sess: SessionState | null;
  inWorkspace: boolean;
}

interface SessionContext extends CommandContext { sess: SessionState }

/** A handler returns false when it turned out to be a no-op. */
type Run<C> = (ctx: C) => boolean | void;

export interface CommandSpec {
  icon: LucideIcon;
  /** Kept out of the command palette. */
  hidden?: boolean;
  /** What must exist before it can act. */
  needs?: 'session' | 'workspace';
  run: (ctx: CommandContext) => boolean;
}

/** True when DOM focus is inside the panel column. */
function isSidebarFocused(): boolean {
  const el = document.activeElement as HTMLElement | null;
  return !!el?.closest('.sidebar-wrapper');
}

/** True when DOM focus is inside the agent's column. */
function isAgentFocused(): boolean {
  const el = document.activeElement as HTMLElement | null;
  return !!el?.closest('.agent-pane');
}

const cmd = (icon: LucideIcon, run: Run<CommandContext>): CommandSpec => ({ icon, run: (ctx) => run(ctx) !== false });

const inSession = (icon: LucideIcon, run: Run<SessionContext>): CommandSpec => ({
  icon,
  needs: 'session',
  run: (ctx) => !!ctx.sess && run({ ...ctx, sess: ctx.sess }) !== false,
});

const inWorkspace = (icon: LucideIcon, run: Run<SessionContext>): CommandSpec => ({
  icon,
  needs: 'workspace',
  run: (ctx) => !!ctx.sess && ctx.inWorkspace && run({ ...ctx, sess: ctx.sess }) !== false,
});

/** Moves session focus around the tab order; no active session starts at either end. */
const stepSession = (d: 1 | -1): Run<CommandContext> => ({ st }) => {
  const order = st.sessionOrder;
  if (!order.length) return false;
  const idx = order.indexOf(st.activeSessionId ?? '');
  const target = idx === -1
    ? (d === 1 ? order[0] : order[order.length - 1])
    : order[(idx + d + order.length) % order.length];
  st.focusSession(target);
};

/** 3-state: closed → open+focus; open, not focused → focus; focused → close. */
const showPanel = (tab: SidebarTab): Run<SessionContext> => ({ st, sess }) => {
  const inCode = sess.workspaceMode === 'code';
  // Fold only when already looking at this panel.
  if (inCode && !sess.sidebarCollapsed && sess.sidebarTab === tab && isSidebarFocused()) {
    sessionActions(sess.id).setSidebarCollapsed(true);
    // Hand the keyboard back to the editor.
    st.updateSession(sess.id, (x) => ({ editorFocusNonce: x.editorFocusNonce + 1 }));
    return;
  }
  st.updateSession(sess.id, () => ({
    sidebarTab: tab,
    sidebarCollapsed: false,
    // Overview is a mode, not a tab.
    workspaceMode: 'code' as const,
    ...(tab === 'git' ? { gitSubTab: 'changes' as GitSubTab } : {}),
  }));
  st.setView('workspace');
  st.requestPanelFocus();
};

/** Opens the files panel and puts the caret in its search box. */
const findInFiles = (mode?: 'text'): Run<SessionContext> => ({ st, sess }) => {
  st.updateSession(sess.id, () => ({ sidebarTab: 'files' }));
  st.setView('workspace');
  st.requestFileSearchFocus(mode);
};

const stepTab = (d: 1 | -1): Run<SessionContext> => ({ sess }) => {
  const pane = sess.panes.find((p) => p.id === sess.activePaneId) ?? sess.panes[0];
  if (!pane || pane.tabs.length < 2) return false;
  const idx = pane.tabs.findIndex((t) => t.id === pane.activeTabId);
  const next = pane.tabs[(idx + d + pane.tabs.length) % pane.tabs.length];
  sessionActions(sess.id).setActiveTab(pane.id, next.id);
};

/** Opens a header picker, then moves its highlight on every further press. */
const cyclePicker = <C extends CommandContext>(
  kind: 'session' | 'repo' | 'worktree',
  count: (ctx: C) => number,
): Run<C> => (ctx) => {
  const { st } = ctx;
  const n = count(ctx);
  if (n === 0) return false;
  if (kind !== 'session') st.setView('workspace');
  if (st.openPicker !== kind) { st.setOpenPicker(kind); return; }
  st.setPickerCursor((st.pickerCursor + 1) % n);
};

const setFont = (next: (current: number) => number): Run<CommandContext> => ({ st }) => {
  const current = st.config?.ui.font_size ?? DEFAULT_FONT_SIZE;
  st.setFontSize(Math.max(FONT_MIN, Math.min(FONT_MAX, next(current))));
};

export const COMMAND_REGISTRY: Record<CommandId, CommandSpec> = {
  'palette.commands': { icon: Command, hidden: true, run: ({ st }) => { st.setCommandPaletteOpen(true); return true; } },
  'files.quickOpen': inSession(FileSearch, findInFiles()),
  'files.search': inSession(Search, findInFiles('text')),
  'settings.open': cmd(Settings, ({ st }) => st.openSettings()),

  // Pressing it while already in overview goes back to the code view.
  'panel.overview': inSession(Compass, ({ st, sess, inWorkspace: inWs }) => {
    sessionActions(sess.id).setWorkspaceMode(sess.workspaceMode === 'overview' && inWs ? 'code' : 'overview');
    st.setView('workspace');
  }),
  'panel.files': inSession(FolderTree, showPanel('files')),
  'panel.git': inSession(GitBranch, showPanel('git')),
  'panel.annotations': inSession(StickyNote, showPanel('annotations')),
  'git.cycleSubTab': inWorkspace(Repeat, ({ st, sess }) => {
    if (sess.sidebarTab !== 'git') return false;
    const tabs: GitSubTab[] = sess.kind === 'explorer'
      ? ['changes', 'commits']
      : ['changes', 'commits', 'forge'];
    const next = tabs[(tabs.indexOf(sess.gitSubTab) + 1) % tabs.length];
    st.updateSession(sess.id, () => ({ gitSubTab: next }));
  }),
  // requestCommitFocus also opens and un-collapses the git panel.
  'git.commitFocus': inSession(GitCommitVertical, ({ st }) => {
    st.setView('workspace');
    st.requestCommitFocus();
  }),

  'view.tasks': cmd(LayoutGrid, ({ st }) => st.setView('home')),
  'view.notifications': cmd(Bell, ({ st }) => st.setNotificationsOpen(!st.notificationsOpen)),
  'agents.sidebar': cmd(Bot, () => toggleAgentsSidebar()),
  'session.next': cmd(ChevronRight, stepSession(1)),
  'session.switcher': cmd(Layers, cyclePicker('session', ({ st }) => st.sessionOrder.length)),
  'session.prev': cmd(ChevronLeft, stepSession(-1)),

  'workspace.toggleTerminal': cmd(SquareTerminal, ({ st, sess, inWorkspace: inWs }) => {
    // On Home the terminal is an app-level dock.
    if (!inWs) {
      const showing = st.terminalConsoleOpen;
      st.setTerminalConsoleOpen(!showing);
      if (!showing) st.requestTerminalFocus();
      return;
    }
    if (!sess) return false;
    toggleTerminal();
  }),
  // 3-state: closed → open+focus; open, not focused → focus; focused → close.
  'agent.console': cmd(PanelRight, ({ st, sess }) => {
    if (st.consoleOpen && isAgentFocused()) {
      st.setConsoleOpen(false);
      return;
    }
    if (sess) st.setView('workspace');
    st.requestConsoleFocus();
  }),
  'pane.splitRight': inWorkspace(Columns2, ({ sess }) => sessionActions(sess.id).splitPane('row')),
  'pane.splitDown': inWorkspace(Rows2, ({ sess }) => sessionActions(sess.id).splitPane('col')),
  'pane.close': inWorkspace(X, ({ sess }) => sessionActions(sess.id).closePane(sess.activePaneId)),
  'pane.next': inWorkspace(ArrowRightLeft, ({ sess }) => sessionActions(sess.id).focusNextPane()),
  'pane.maximize': inWorkspace(Maximize2, ({ st, sess }) => {
    // Focus inside the agent maximizes the agent column.
    if (isAgentFocused()) {
      st.setAgentMaximized(!st.agentMaximized);
      return;
    }
    sessionActions(sess.id).toggleMaximizePane();
  }),
  'tab.next': inWorkspace(ChevronRight, stepTab(1)),
  'tab.prev': inWorkspace(ChevronLeft, stepTab(-1)),
  'tab.close': inWorkspace(X, ({ sess }) => {
    const pane = sess.panes.find((p) => p.id === sess.activePaneId) ?? sess.panes[0];
    if (!pane?.activeTabId) return false;
    sessionActions(sess.id).closeTab(pane.id, pane.activeTabId);
  }),
  'repo.switch': inSession(FolderGit2, cyclePicker('repo', ({ sess }) => sess.repos.length)),
  'worktree.switch': inSession(GitBranch, cyclePicker('worktree', ({ sess }) =>
    sess.worktrees.filter((w) => w.repo_id === sess.activeRepoId).length)),
  'repo.add': inSession(FolderPlus, ({ st }) => {
    st.setView('workspace');
    st.setAddRepoOpen(true);
  }),

  'editor.focus': inSession(SquarePen, ({ st, sess }) => {
    st.updateSession(sess.id, (s) => ({ editorFocusNonce: s.editorFocusNonce + 1 }));
    st.setView('workspace');
  }),
  'editor.toggleVim': cmd(Keyboard, ({ st }) => st.setVimMode(!st.vimMode)),
  'editor.toggleBlame': inSession(Fingerprint, ({ st, sess }) =>
    st.updateSession(sess.id, (s) => ({ blameOn: !s.blameOn }))),
  'font.increase': cmd(ZoomIn, setFont((c) => c + 1)),
  'font.decrease': cmd(ZoomOut, setFont((c) => c - 1)),
  'font.reset': cmd(RotateCcw, setFont(() => DEFAULT_FONT_SIZE)),
};

export function commandContext(): CommandContext {
  const st = useStore.getState();
  const sess = st.activeSessionId ? st.sessions[st.activeSessionId] ?? null : null;
  return { st, sess, inWorkspace: st.view === 'workspace' };
}

/** Runs a global command. Returns false when it is a no-op in this context. */
export function runCommand(id: CommandId): boolean {
  return COMMAND_REGISTRY[id].run(commandContext());
}

export interface CommandRow {
  id: CommandId;
  label: string;
  group: string;
  icon: LucideIcon;
  shortcut?: string;
}

/** Keybound commands the palette offers here, in the order the keymap declares them. */
export function commandRows(keymap: Keymap, avail: { session: boolean; workspace: boolean }): CommandRow[] {
  const rows: CommandRow[] = [];
  for (const b of COMMANDS) {
    const spec = COMMAND_REGISTRY[b.id];
    if (spec.hidden) continue;
    if (spec.needs === 'session' && !avail.session) continue;
    if (spec.needs === 'workspace' && !(avail.session && avail.workspace)) continue;
    rows.push({
      id: b.id,
      label: b.label,
      group: b.group,
      icon: spec.icon,
      shortcut: shortcutLabel(keymap, b.id),
    });
  }
  return rows;
}
