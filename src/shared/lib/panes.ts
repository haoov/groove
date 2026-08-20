import { invoke } from '../ipc/invoke';
import { useStore, sessionActions, isTerminalPane, type EditorTab, type SessionState } from '../store';
import { disposeHost } from './terminalHost';

/** True when DOM focus is currently inside a terminal tab. */
function isTerminalFocused(): boolean {
  const el = document.activeElement as HTMLElement | null;
  return !!el?.closest('[data-dock="terminal"]');
}

function activeSession(): SessionState | null {
  const st = useStore.getState();
  return st.activeSessionId ? st.sessions[st.activeSessionId] ?? null : null;
}

function findTab(
  sess: SessionState,
  pred: (t: EditorTab) => boolean,
): { paneId: string; tab: EditorTab } | null {
  for (const p of sess.panes) {
    const tab = p.tabs.find(pred);
    if (tab) return { paneId: p.id, tab };
  }
  return null;
}

/**
 * Open (or focus) a terminal tab.
 *
 * Terminals live in ONE dock at the bottom of the workspace, never in an editor
 * pane: the first one splits the root so the dock spans the full width, and every
 * one after joins it as a tab. Side-by-side terminals come from splitting the dock
 * itself. A PTY whose tab was closed re-binds to the new tab instead of leaking.
 */
export function ensureTerminalTab(opts?: { fresh?: boolean }): void {
  const st = useStore.getState();
  const sess = activeSession();
  if (!sess) return;
  const actions = sessionActions(sess.id);

  const dock = sess.panes.find(isTerminalPane);
  const last = dock ? dock.tabs[dock.tabs.length - 1] : null;

  if (!opts?.fresh && dock && last) {
    // Focus it in place — NEVER a new openTab for an existing terminal (a second
    // tab bound to the same session would steal its xterm element).
    actions.setActiveTab(dock.id, last.id);
    st.requestTerminalFocus();
    return;
  }

  // A running session whose tab was closed re-binds to the new tab.
  const boundIds = new Set(
    sess.panes.flatMap((p) => p.tabs.filter((t) => t.kind === 'terminal').map((t) => t.ptySessionId)),
  );
  const unbound = sess.ptySessions.find((p) => p.ptyType === 'terminal' && !boundIds.has(p.sessionId));
  const tab = {
    repoId: '', filePath: '', view: 'diff' as const, kind: 'terminal' as const,
    ptySessionId: unbound?.sessionId,
    label: unbound?.label ?? 'Terminal',
  };

  if (dock) {
    actions.openTab(tab, { paneId: dock.id });
  } else {
    actions.splitRootPane('col', 0.72); // full-width bottom dock, focused
    actions.openTab(tab);
  }
  st.requestTerminalFocus();
}

/** 3-state toggle (matches the old dock feel): none → open+focus; open but not
 *  focused → focus; focused → close the tab (the PTY keeps running). */
export function toggleTerminal(): void {
  const sess = activeSession();
  if (!sess) return;
  const existing = findTab(sess, (t) => t.kind === 'terminal');
  if (!existing || !isTerminalFocused()) {
    ensureTerminalTab();
    return;
  }
  sessionActions(sess.id).closeTab(existing.paneId, existing.tab.id);
}

/** Kill a PTY tab's session for real (context-menu action): stop the backend
 *  PTY, dispose the terminal, drop the store row, close the tab. */
export async function killPtyTab(sessionKey: string, paneId: string, tab: EditorTab): Promise<void> {
  const ptySessionId = tab.ptySessionId;
  if (ptySessionId) {
    try {
      await invoke('stop_agent_session', { sessionId: ptySessionId });
    } catch { /* already dead — clean up anyway */ }
    disposeHost(ptySessionId);
    sessionActions(sessionKey).removePtySession(ptySessionId);
  }
  sessionActions(sessionKey).closeTab(paneId, tab.id);
}
