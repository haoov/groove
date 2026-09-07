import { useEffect } from 'react';
import { useStore, sessionActions, type GitSubTab, type SidebarTab } from '../../shared/store';
import { toggleTerminal } from '../../shared/lib/panes';
import { toggleAgentsSidebar } from '../../shared/lib/agentsSidebar';
import { DEFAULT_FONT_SIZE, FONT_MIN, FONT_MAX } from '../../shared/ipc/ipc';
import { COMMANDS, type CommandId } from '../../shared/lib/keybindings';
import { chordMatches, isModifierOnly, isTypingCharacter } from '../../shared/lib/keys';

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

/** Runs a global command. Returns false when it is a no-op in this context. */
export function runCommand(id: CommandId): boolean {
  const st = useStore.getState();
  const sid = st.activeSessionId;
  const sess = sid ? st.sessions[sid] : null;
  const inWorkspace = st.view === 'workspace';

  switch (id) {
    case 'palette.commands':
      st.setCommandPaletteOpen(true);
      return true;
    case 'settings.open':
      st.openSettings();
      return true;
    case 'view.tasks':
      st.setView('home');
      return true;
    case 'agents.sidebar':
      return toggleAgentsSidebar();
    case 'view.notifications':
      st.setNotificationsOpen(!st.notificationsOpen);
      return true;
    case 'editor.toggleVim':
      st.setVimMode(!st.vimMode);
      return true;

    case 'files.quickOpen':
      if (!sess) return false;
      st.updateSession(sess.id, () => ({ sidebarTab: 'files' }));
      st.setView('workspace');
      st.requestFileSearchFocus();
      return true;
    case 'files.search':
      if (!sess) return false;
      st.updateSession(sess.id, () => ({ sidebarTab: 'files' }));
      st.setView('workspace');
      st.requestFileSearchFocus('text');
      return true;

    case 'session.next':
    case 'session.prev': {
      const order = st.sessionOrder;
      if (!order.length) return false;
      const idx = order.indexOf(st.activeSessionId ?? '');
      let target: string;
      if (idx === -1) {
        // No active session: next lands on the first, prev on the last.
        target = id === 'session.next' ? order[0] : order[order.length - 1];
      } else {
        const d = id === 'session.next' ? 1 : -1;
        target = order[(idx + d + order.length) % order.length];
      }
      st.focusSession(target);
      return true;
    }

    // Overview mode. Pressing it while already there goes back to the code view.
    case 'panel.overview': {
      if (!sess) return false;
      const a = sessionActions(sess.id);
      a.setWorkspaceMode(sess.workspaceMode === 'overview' && inWorkspace ? 'code' : 'overview');
      st.setView('workspace');
      return true;
    }

    // 3-state: closed → open+focus; open, not focused → focus; focused → close.
    case 'panel.files':
    case 'panel.git':
    case 'panel.annotations': {
      if (!sess) return false;
      const tab: SidebarTab =
        id === 'panel.files' ? 'files' : id === 'panel.git' ? 'git' : 'annotations';
      const inCode = sess.workspaceMode === 'code';
      // Fold only when already looking at this panel.
      if (inCode && !sess.sidebarCollapsed && sess.sidebarTab === tab && isSidebarFocused()) {
        sessionActions(sess.id).setSidebarCollapsed(true);
        // Hand the keyboard back to the editor.
        st.updateSession(sess.id, (x) => ({ editorFocusNonce: x.editorFocusNonce + 1 }));
        return true;
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
      return true;
    }

    // 3-state: closed → open+focus; open, not focused → focus; focused → close.
    case 'agent.console':
      if (st.consoleOpen && isAgentFocused()) {
        st.setConsoleOpen(false);
        return true;
      }
      if (sess) st.setView('workspace');
      st.requestConsoleFocus();
      return true;
    case 'workspace.toggleTerminal':
      // On Home the terminal is an app-level dock.
      if (!inWorkspace) {
        const showing = st.terminalConsoleOpen;
        st.setTerminalConsoleOpen(!showing);
        if (!showing) st.requestTerminalFocus();
        return true;
      }
      if (!sess) return false;
      toggleTerminal();
      return true;

    case 'pane.splitRight':
    case 'pane.splitDown':
      if (!sess || !inWorkspace) return false;
      sessionActions(sess.id).splitPane(id === 'pane.splitRight' ? 'row' : 'col');
      return true;
    case 'pane.close': {
      if (!sess || !inWorkspace) return false;
      sessionActions(sess.id).closePane(sess.activePaneId);
      return true;
    }
    case 'pane.next':
      if (!sess || !inWorkspace) return false;
      sessionActions(sess.id).focusNextPane();
      return true;
    case 'pane.maximize':
      // Focus inside the agent maximizes the agent column, in a workspace only.
      if (isAgentFocused() && inWorkspace) {
        st.setAgentMaximized(!st.agentMaximized);
        return true;
      }
      if (!sess || !inWorkspace) return false;
      sessionActions(sess.id).toggleMaximizePane();
      return true;

    // File tabs inside the focused pane.
    case 'tab.next':
    case 'tab.prev': {
      if (!sess || !inWorkspace) return false;
      const pane = sess.panes.find((p) => p.id === sess.activePaneId) ?? sess.panes[0];
      if (!pane || pane.tabs.length < 2) return false;
      const idx = pane.tabs.findIndex((t) => t.id === pane.activeTabId);
      const d = id === 'tab.next' ? 1 : -1;
      const next = pane.tabs[(idx + d + pane.tabs.length) % pane.tabs.length];
      sessionActions(sess.id).setActiveTab(pane.id, next.id);
      return true;
    }
    case 'tab.close': {
      if (!sess || !inWorkspace) return false;
      const pane = sess.panes.find((p) => p.id === sess.activePaneId) ?? sess.panes[0];
      if (!pane?.activeTabId) return false;
      sessionActions(sess.id).closeTab(pane.id, pane.activeTabId);
      return true;
    }

    // Alt+S: the first press opens the session switcher; each further press moves the highlight.
    case 'session.switcher': {
      const n = st.sessionOrder.length;
      if (n === 0) return false;
      if (st.openPicker !== 'session') { st.setOpenPicker('session'); return true; }
      st.setPickerCursor((st.pickerCursor + 1) % n);
      return true;
    }

    case 'git.commitFocus':
      // requestCommitFocus also opens and un-collapses the git panel.
      if (!sess) return false;
      st.setView('workspace');
      st.requestCommitFocus();
      return true;

    // Alt+R: open the header repo switcher, then move the highlight.
    case 'repo.switch': {
      if (!sess || sess.repos.length === 0) return false;
      st.setView('workspace');
      if (st.openPicker !== 'repo') { st.setOpenPicker('repo'); return true; }
      st.setPickerCursor((st.pickerCursor + 1) % sess.repos.length);
      return true;
    }

    // Alt+W: open the header worktree switcher, then move the highlight.
    case 'worktree.switch': {
      if (!sess) return false;
      const wts = sess.worktrees.filter((w) => w.repo_id === sess.activeRepoId);
      if (wts.length === 0) return false;
      st.setView('workspace');
      if (st.openPicker !== 'worktree') { st.setOpenPicker('worktree'); return true; }
      st.setPickerCursor((st.pickerCursor + 1) % wts.length);
      return true;
    }

    case 'repo.add':
      if (!sess) return false;
      st.setView('workspace');
      st.setAddRepoOpen(true);
      return true;

    case 'editor.toggleBlame':
      if (!sess) return false;
      st.updateSession(sess.id, (s) => ({ blameOn: !s.blameOn }));
      return true;

    case 'font.increase':
    case 'font.decrease':
    case 'font.reset': {
      const current = st.config?.ui.font_size ?? DEFAULT_FONT_SIZE;
      const next =
        id === 'font.reset' ? DEFAULT_FONT_SIZE
        : id === 'font.increase' ? current + 1
        : current - 1;
      st.setFontSize(Math.max(FONT_MIN, Math.min(FONT_MAX, next)));
      return true;
    }

    case 'editor.focus':
      if (!sess) return false;
      st.updateSession(sess.id, (s) => ({ editorFocusNonce: s.editorFocusNonce + 1 }));
      st.setView('workspace');
      return true;

    case 'git.cycleSubTab': {
      if (!sess || !inWorkspace || sess.sidebarTab !== 'git') return false;
      const tabs: GitSubTab[] = sess.kind === 'explorer'
        ? ['changes', 'commits']
        : ['changes', 'commits', 'forge'];
      const idx = tabs.indexOf(sess.gitSubTab);
      const next = tabs[(idx + 1) % tabs.length];
      st.updateSession(sess.id, () => ({ gitSubTab: next }));
      return true;
    }
  }
  return false;
}

/** Installs the global keydown listener in the capture phase; app shortcuts win over CodeMirror/xterm. */
export function useKeybindings() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Let typed characters through.
      if (isModifierOnly(e) || isTypingCharacter(e)) return;
      const st = useStore.getState();
      if (st.capturingKey) return; // Settings is rebinding.
      const { keymap } = st;

      const el = e.target as HTMLElement | null;
      const tag = el?.tagName;
      const inField = tag === 'INPUT' || tag === 'TEXTAREA' || !!el?.isContentEditable;

      // The file-search input owns Ctrl+J/K; stopPropagation cannot reach the capture phase.
      if (el?.dataset?.fileSearch === '1' && (e.ctrlKey || e.metaKey) && (e.key === 'j' || e.key === 'k')) return;

      // The repo picker's filter owns Ctrl+J/K and Ctrl+Tab.
      if (el?.dataset?.repoPicker === '1' && (e.ctrlKey || e.metaKey)) return;

      // A terminal owns Ctrl+Shift+C/V; the capture phase would run `editor.focus` first.
      if (
        (e.ctrlKey || e.metaKey) && e.shiftKey &&
        (e.code === 'KeyC' || e.code === 'KeyV') &&
        el?.closest('.pty-pane')
      ) return;

      for (const cmd of COMMANDS) {
        for (const c of keymap[cmd.id] ?? []) {
          if (!chordMatches(e, c)) continue;
          // Do not steal un-modified keys from text fields.
          if (inField && !c.alt && !c.ctrl) return;
          e.preventDefault();
          e.stopPropagation();
          runCommand(cmd.id);
          return;
        }
      }
    };
    // Mouse back/forward (M4/M5) cycle the focused pane; preventDefault stops history navigation.
    const onMouse = (e: MouseEvent) => {
      if (e.button !== 3 && e.button !== 4) return;
      const st = useStore.getState();
      if (st.view !== 'workspace') return;
      const sess = st.activeSessionId ? st.sessions[st.activeSessionId] : null;
      if (!sess || sess.panes.length < 2) return;
      e.preventDefault();
      const order = sess.panes.map((p) => p.id);
      const idx = order.indexOf(sess.activePaneId);
      const dir = e.button === 3 ? -1 : 1;
      const target = order[(idx + dir + order.length) % order.length];
      sessionActions(sess.id).focusPane(target);
    };
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('mousedown', onMouse, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('mousedown', onMouse, true);
    };
  }, []);
}
