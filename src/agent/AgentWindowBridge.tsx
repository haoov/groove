import { useEffect } from 'react';
import { emitTo, listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useStore, sessionActions, type AppState } from '../shared/store';
import { ensureAgentSession, reloadAgent, sendSkill } from '../shared/lib/agentSend';
import { buildAgentRows, orderedSessions } from '../shared/lib/agents';
import { endSession } from '../shared/lib/endSession';
import { goToSessionById } from '../shared/lib/goToSession';
import { isMac } from '../shared/lib/platform';
import {
  AGENT_WINDOW_LABEL,
  BRIDGE,
  DEFAULT_WINDOW_SIZE,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
  readBounds,
  sessionScopeError,
  writeBounds,
  type AgentWindowState,
  type CommandDone,
  type CommandEnvelope,
} from '../shared/lib/agentWindow';

/**
 * The main window's half of the detached agent window: opens and closes it, mirrors the
 * agent state into it, and runs the commands it sends back. The window never mounts useIpc.
 */
export function AgentWindowBridge() {
  const detached = useStore((s) => s.agentDetached);
  const focusNonce = useStore((s) => s.consoleFocusNonce);

  // Window lifecycle follows the flag; the OS close button docks back.
  useEffect(() => {
    if (!detached) {
      WebviewWindow.getByLabel(AGENT_WINDOW_LABEL)
        .then((w) => w?.close())
        .catch(() => {});
      return;
    }
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    openAgentWindow()
      .then(async (win) => {
        const u = await win.once('tauri://destroyed', () => {
          useStore.getState().setAgentDetached(false);
        });
        if (cancelled) u();
        else unlisten = u;
      })
      .catch((e) => {
        useStore.getState().setLastError(`Agent window: ${String(e)}`);
        useStore.getState().setAgentDetached(false);
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [detached]);

  // State mirror: push on every store change and again on the window's READY.
  useEffect(() => {
    if (!detached) return;
    let last = '';
    const push = () => {
      const state = buildState(useStore.getState());
      const json = JSON.stringify(state);
      if (json === last) return;
      last = json;
      emitTo(AGENT_WINDOW_LABEL, BRIDGE.STATE, state).catch(() => {});
    };
    const unsub = useStore.subscribe(push);
    push();
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen(BRIDGE.READY, () => {
      last = '';
      push();
    }).then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });
    return () => {
      cancelled = true;
      unsub();
      unlisten?.();
    };
  }, [detached]);

  // Commands, each answered with DONE.
  useEffect(() => {
    if (!detached) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen<CommandEnvelope>(BRIDGE.COMMAND, async ({ payload }) => {
      let error: string | null = null;
      try {
        await runCommand(payload);
      } catch (e) {
        error = String(e);
        useStore.getState().setLastError(error);
      }
      const done: CommandDone = { nonce: payload.nonce, error };
      emitTo(AGENT_WINDOW_LABEL, BRIDGE.DONE, done).catch(() => {});
    }).then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [detached]);

  // Focus the window on the agent shortcut while detached.
  useEffect(() => {
    if (!detached || focusNonce === 0) return;
    WebviewWindow.getByLabel(AGENT_WINDOW_LABEL)
      .then((w) => w?.setFocus())
      .catch(() => {});
  }, [detached, focusNonce]);

  return null;
}

async function openAgentWindow(): Promise<WebviewWindow> {
  const existing = await WebviewWindow.getByLabel(AGENT_WINDOW_LABEL);
  if (existing) return existing;
  const bounds = readBounds();
  const mac = isMac();
  const win = new WebviewWindow(AGENT_WINDOW_LABEL, {
    url: 'index.html',
    title: 'Groove agent',
    width: bounds?.width ?? DEFAULT_WINDOW_SIZE.width,
    height: bounds?.height ?? DEFAULT_WINDOW_SIZE.height,
    x: bounds?.x,
    y: bounds?.y,
    minWidth: MIN_WINDOW_WIDTH,
    minHeight: MIN_WINDOW_HEIGHT,
    // Same chrome rule as the main window (tauri.conf.json / tauri.macos.conf.json).
    decorations: mac,
    ...(mac ? { titleBarStyle: 'overlay' as const, hiddenTitle: true } : {}),
  });
  await new Promise<void>((resolve, reject) => {
    win.once('tauri://created', () => resolve()).catch(reject);
    win.once<string>('tauri://error', (e) => reject(new Error(String(e.payload)))).catch(reject);
  });
  return win;
}

function buildState(s: AppState): AgentWindowState {
  const sess = s.activeSessionId ? s.sessions[s.activeSessionId] : null;
  const taskId = sess?.task?.short_id ?? null;
  return {
    sessionId: s.activeSessionId,
    taskId,
    ptyId: sess?.ptySessions.find((p) => p.ptyType === 'agent')?.sessionId ?? null,
    kind: sess?.kind ?? 'task',
    activity: taskId ? s.agentActivity[taskId] ?? null : null,
    autoApprove: sess?.autoApprove ?? false,
    skills: s.skills,
    skillsStale: s.skillsStale,
    config: s.config,
    agents: buildAgentRows(orderedSessions(s.sessions, s.sessionOrder), s.activeSessionId, s.agentActivity),
    agentsOpen: s.agentsSidebarOpen,
  };
}

async function runCommand(envelope: CommandEnvelope): Promise<void> {
  const st = useStore.getState();
  const drifted = sessionScopeError(envelope, st.activeSessionId);
  if (drifted) throw new Error(drifted);
  const sessionKey = envelope.sessionId;
  const cmd = envelope.command;
  switch (cmd.type) {
    case 'dock':
      st.setAgentDetached(false);
      return;
    case 'bounds':
      writeBounds(cmd.bounds);
      return;
    case 'agentsOpen':
      st.setAgentsSidebarOpen(cmd.value);
      return;
    case 'goToSession':
      if (!goToSessionById(cmd.sessionId, { agent: true })) throw new Error('that session is closed');
      return;
    case 'closeSession':
      await endSession(cmd.sessionId);
      return;
    case 'autoApprove':
      if (sessionKey) sessionActions(sessionKey).setAutoApprove(cmd.value);
      return;
    case 'start':
      if (!sessionKey) throw new Error('no session is focused');
      await ensureAgentSession(sessionKey);
      return;
    case 'skill':
      if (!sessionKey) throw new Error('no session is focused');
      await sendSkill(sessionKey, cmd.skillId, cmd.args);
      return;
    case 'reload':
      if (!sessionKey) throw new Error('no session is focused');
      await reloadAgent(sessionKey);
      return;
  }
}
