import { useCallback, useEffect, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useIpc } from './providers/useIpc';
import { useKeybindings } from './providers/useKeybindings';
import { useTaskTimer } from '../sessions/useTaskTimer';
import { useStore } from '../shared/store';
import { FirstRun } from '../setup/FirstRun';
import { Header } from './chrome/Header';
import { ActivityRail } from './chrome/ActivityRail';
import { StatusBar } from './chrome/StatusBar';
import { Home } from '../home';
import { SessionWorkspaces } from '../workspace/SessionWorkspaces';
import { ConfirmModal } from '../approvals/ConfirmModal';
import { CommandPalette } from '../command/CommandPalette';
import { TaskOpenWizard } from '../setup/TaskOpenWizard';
import { AddRepoModal } from '../setup/AddRepoModal';
import { ResizeHandles } from './chrome/ResizeHandles';
import { SettingsModal } from '../setup/SettingsModal';
import { Toasts } from '../notifications/Toasts';
import { AgentConsole } from '../agent/AgentConsole';
import { TerminalConsole } from '../terminal/TerminalConsole';
import { applyTheme, applyFontSize, applyFontFamily } from '../shared/lib/theme';
import { DEFAULT_FONT_SIZE, DEFAULT_THEME, type Config } from '../shared/ipc/ipc';

/** Refresh the review queue on startup and every ~5 min (rail badge + strip). */
const REVIEW_POLL_MS = 5 * 60 * 1000;
function useReviewQueue() {
  const refreshReviewQueue = useStore((s) => s.refreshReviewQueue);
  useEffect(() => {
    refreshReviewQueue();
    const t = setInterval(refreshReviewQueue, REVIEW_POLL_MS);
    return () => clearInterval(t);
  }, [refreshReviewQueue]);
}

/**
 * Keep the backend's active task pointed at the focused session. Every MCP tool
 * resolves its target from it (get_active_task / get_worktrees / get_task_diff /
 * annotations / the push guard / create_task_from_explorer), and opening a task
 * is not the only way the focus changes — clicking a session tab, closing one,
 * or an explorer→task conversion all move it. Null when no session is open.
 */
function useActiveTaskSync() {
  const activeShortId = useStore((s) =>
    s.activeSessionId ? s.sessions[s.activeSessionId]?.task?.short_id ?? null : null,
  );
  useEffect(() => {
    invoke('set_active_task', { shortId: activeShortId }).catch(console.warn);
  }, [activeShortId]);
}

/**
 * Refresh the Home snapshot whenever Home becomes visible or the set of open
 * sessions changes. Edits and landed git ops refresh it too (useIpc), and both
 * paths no-op while a workspace is showing — Home isn't rendered then, so its
 * git calls would be pure waste.
 */
function useHomeSnapshot() {
  const visible = useStore((s) => s.view !== 'workspace');
  const sessionOrder = useStore((s) => s.sessionOrder);
  const refreshHome = useStore((s) => s.refreshHome);
  useEffect(() => {
    if (visible) refreshHome();
  }, [visible, sessionOrder, refreshHome]);
}

export default function App() {
  useIpc();
  useKeybindings();
  useReviewQueue();
  useActiveTaskSync();
  useHomeSnapshot();
  useTaskTimer();

  const view = useStore((s) => s.view);
  const setConfig = useStore((s) => s.setConfig);
  const setLastError = useStore((s) => s.setLastError);
  const hydrateAgentActivity = useStore((s) => s.hydrateAgentActivity);
  // Mounted once here rather than twice with local state, so Alt+R and both
  // buttons open the same instance.
  const addRepoOpen = useStore((s) => s.addRepoOpen);
  const setAddRepoOpen = useStore((s) => s.setAddRepoOpen);

  // Agent state lives in memory on the backend, so after a reload the app knows
  // nothing until the next hook fires — ask once for whatever is already known.
  useEffect(() => {
    hydrateAgentActivity();
  }, [hydrateAgentActivity]);

  // Three states, not two: loading, configured, and never-configured. Without the
  // third, a new machine showed an empty Home and a Notion error in the corner.
  const [configured, setConfigured] = useState<boolean | null>(null);

  const applyConfig = useCallback((cfg: Config) => {
    setConfig(cfg);
    applyFontSize(cfg.ui?.font_size ?? DEFAULT_FONT_SIZE);
    applyFontFamily(cfg.ui?.font_family);
    applyTheme(cfg.ui?.theme ?? DEFAULT_THEME);
    setConfigured(true);
  }, [setConfig]);

  useEffect(() => {
    invoke<Config | null>('get_config')
      .then((cfg) => {
        if (!cfg) { setConfigured(false); return; }
        applyConfig(cfg);
      })
      .catch((e) => {
        // An unreadable config is a setup problem, so it goes to the setup screen
        // (which prints the path and the parse error) rather than a toast.
        setConfigured(false);
        setLastError(`Failed to load config: ${String(e)}`);
      });
  }, [applyConfig, setLastError]);

  if (configured === null) return <div className="app app-booting" />;
  if (!configured) return <FirstRun onReady={applyConfig} />;

  return (
    <div className="app">
      <Header />
      <div className="app-body">
        <ActivityRail />
        <main className="app-main">
          {/* Home (shown when no session is focused): reviews / tasks / explorers. */}
          {view !== 'workspace' && <Home />}
          {/* Active session workspace — kept mounted across views so
              background sessions' terminals persist. */}
          <SessionWorkspaces hidden={view !== 'workspace'} />
        </main>
        {/* The agent's own column, between the work and the session list, so the
            two right-hand columns read as one edge. It addresses the focused
            session; Home is a pure dashboard — no agent there. */}
        {view === 'workspace' && <AgentConsole />}
      </div>
      {/* A scratch shell on Home (session-less — it owns its PTY). In a
          workspace the panes own the terminals, so this is not mounted there. */}
      {view !== 'workspace' && <TerminalConsole />}
      <StatusBar />

      {/* Overlays */}
      <ConfirmModal />
      <CommandPalette />
      <TaskOpenWizard />
      {addRepoOpen && <AddRepoModal onClose={() => setAddRepoOpen(false)} />}
      <SettingsModal />
      <Toasts />

      {/* Frameless-window resize grips (must be last so they sit on top) */}
      <ResizeHandles />
    </div>
  );
}
