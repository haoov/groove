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
import { AddRepoModal } from '../setup/AddRepoModal';
import { AddWorktreeModal } from '../setup/AddWorktreeModal';
import { ResizeHandles } from './chrome/ResizeHandles';
import { SettingsView } from '../settings/SettingsView';
import { Toasts } from '../notifications/Toasts';
import { AgentConsole } from '../agent/AgentConsole';
import { AgentWindowBridge } from '../agent/AgentWindowBridge';
import { TerminalConsole } from '../terminal/TerminalConsole';
import { applyTheme, applyFontSize, applyFontFamily } from '../shared/lib/theme';
import { isMac } from '../shared/lib/platform';
import { DEFAULT_FONT_SIZE, DEFAULT_THEME, type Config } from '../shared/ipc/ipc';

/** Refreshes the review queue on startup and every ~5 min. */
const REVIEW_POLL_MS = 5 * 60 * 1000;
function useReviewQueue() {
  const refreshReviewQueue = useStore((s) => s.refreshReviewQueue);
  useEffect(() => {
    refreshReviewQueue();
    const t = setInterval(refreshReviewQueue, REVIEW_POLL_MS);
    return () => clearInterval(t);
  }, [refreshReviewQueue]);
}

/** Keeps the backend's active task pointed at the focused session. Null when no session is open. */
function useActiveTaskSync() {
  const activeShortId = useStore((s) =>
    s.activeSessionId ? s.sessions[s.activeSessionId]?.task?.short_id ?? null : null,
  );
  useEffect(() => {
    invoke('set_active_task', { shortId: activeShortId }).catch(console.warn);
  }, [activeShortId]);
}

/** Refreshes the Home snapshot when Home becomes visible or the set of open sessions changes. */
function useHomeSnapshot() {
  const visible = useStore((s) => s.view === 'home');
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
  const loadSkills = useStore((s) => s.loadSkills);
  const setLastError = useStore((s) => s.setLastError);
  const hydrateAgentActivity = useStore((s) => s.hydrateAgentActivity);
  const addRepoOpen = useStore((s) => s.addRepoOpen);
  const addWorktreeOpen = useStore((s) => s.addWorktreeOpen);
  const setAddRepoOpen = useStore((s) => s.setAddRepoOpen);
  const setAddWorktreeOpen = useStore((s) => s.setAddWorktreeOpen);

  // Hydrate agent state once after a reload.
  useEffect(() => {
    hydrateAgentActivity();
  }, [hydrateAgentActivity]);

  // Three states: loading, configured, never-configured.
  const [configured, setConfigured] = useState<boolean | null>(null);

  const applyConfig = useCallback((cfg: Config) => {
    setConfig(cfg);
    applyFontSize(cfg.ui?.font_size ?? DEFAULT_FONT_SIZE);
    applyFontFamily(cfg.ui?.font_family);
    applyTheme(cfg.ui?.theme ?? DEFAULT_THEME);
    void loadSkills();
    setConfigured(true);
  }, [setConfig, loadSkills]);

  useEffect(() => {
    invoke<Config | null>('get_config')
      .then((cfg) => {
        if (!cfg) { setConfigured(false); return; }
        applyConfig(cfg);
      })
      .catch((e) => {
        // An unreadable config goes to the setup screen, not a toast.
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
          {view === 'home' && <Home />}
          {view === 'settings' && <SettingsView />}
          {/* Kept mounted across views: background sessions' terminals live here. */}
          <SessionWorkspaces hidden={view !== 'workspace'} />
        </main>
        {view === 'workspace' && <AgentConsole />}
      </div>
      {/* A session-less scratch shell on Home; it owns its PTY. */}
      {view === 'home' && <TerminalConsole />}
      <StatusBar />

      {/* The detached agent window, when there is one. */}
      <AgentWindowBridge />

      {/* Overlays */}
      <ConfirmModal />
      <CommandPalette />
      {addRepoOpen && <AddRepoModal onClose={() => setAddRepoOpen(false)} />}
      {addWorktreeOpen && <AddWorktreeModal onClose={() => setAddWorktreeOpen(false)} />}
      <Toasts />

      {/* Resize grips. Keep them last so they sit on top. Not on macOS: the window is decorated. */}
      {!isMac() && <ResizeHandles />}
    </div>
  );
}
