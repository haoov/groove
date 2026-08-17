import { useEffect, useRef, useState } from 'react';
import { Loader2, Play, ShieldOff, X } from 'lucide-react';
import { useStore, useSession } from '../store';
import { ensureAgentSession, sendToAgent } from '../lib/agentSend';
import { actionsFor } from '../lib/prompts';
import { focusHost } from './terminalHost';
import { useAttachedHost } from './useAttachedHost';
import type { AgentActivity } from '../types/ipc';

/**
 * The agent, as a full-height column on the right of the work.
 *
 * It shows the agent's REAL terminal rather than a re-implementation: no second
 * input to keep in sync, no guessing what Claude is asking, and no state where
 * typing is unsafe, because you are looking at and typing into the actual thing.
 *
 * A column rather than the floating card it used to be — a conversation you keep
 * glancing at wants the height, and a card that hovered over the editor was in the
 * way. It sits between the workspace and the session dock, so the two right-hand
 * columns read as one edge.
 *
 * It addresses whichever session its context names — the focused one in a
 * workspace, the desk on Home (App.tsx picks).
 */

const MIN_WIDTH = 320;
const MAX_WIDTH = 900;
const DEFAULT_WIDTH = 460;
const WIDTH_KEY = 'wb.agentPaneWidth';

export function AgentConsole() {
  const sessionKey = useSession((s) => s.id);
  const activeTask = useSession((s) => s.activeTask);
  const ptySessions = useSession((s) => s.ptySessions);
  const kind = useSession((s) => s.kind);
  const autoApprove = useSession((s) => s.autoApprove);
  const setAutoApprove = useSession((s) => s.setAutoApprove);
  const repos = useSession((s) => s.activeRepos);
  const mrs = useSession((s) => s.mrs);
  const open = useStore((s) => s.consoleOpen);
  const setOpen = useStore((s) => s.setConsoleOpen);
  const focusNonce = useStore((s) => s.consoleFocusNonce);
  const activity = useStore((s) =>
    activeTask ? s.agentActivity[activeTask.short_id] ?? null : null,
  );
  const setLastError = useStore((s) => s.setLastError);
  const maximized = useStore((s) => s.agentMaximized);

  const [starting, setStarting] = useState(false);
  const [sending, setSending] = useState<string | null>(null);
  const [width, setWidth] = useState(() => {
    const saved = Number(localStorage.getItem(WIDTH_KEY));
    return Number.isFinite(saved) && saved >= MIN_WIDTH ? saved : DEFAULT_WIDTH;
  });
  const termRef = useRef<HTMLDivElement>(null);
  const paneRef = useRef<HTMLElement>(null);

  const agentPty = ptySessions.find((p) => p.ptyType === 'agent')?.sessionId ?? null;
  const visible = !!activeTask && open;

  const start = () => {
    if (starting) return;
    setStarting(true);
    ensureAgentSession(sessionKey)
      .catch((e) => setLastError(String(e)))
      .finally(() => setStarting(false));
  };

  // Opening the pane IS the request to start an agent. Only on the open
  // TRANSITION: with it already open, flipping through sessions must not spawn a
  // Claude process in each one, and a failed start must not retry in a loop.
  const wasOpen = useRef(false);
  useEffect(() => {
    const justOpened = open && !wasOpen.current;
    wasOpen.current = open;
    if (!justOpened || !activeTask || agentPty) return;
    start();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, activeTask, agentPty]);

  // Attach the terminal only while actually showing it.
  const holding = visible ? agentPty : null;
  useAttachedHost(holding, termRef);

  useEffect(() => {
    if (holding) focusHost(holding);
  }, [focusNonce, holding]);

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    // The rendered width, not the stored one: in a window too narrow for every
    // column this has been shrunk, and dragging from the stored value would jump.
    const startWidth = paneRef.current?.getBoundingClientRect().width ?? width;
    let latest = startWidth;
    const move = (ev: MouseEvent) => {
      // The handle is on the pane's inner edge, so dragging left widens it.
      latest = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth + (startX - ev.clientX)));
      setWidth(latest);
    };
    const up = () => {
      document.removeEventListener('mousemove', move);
      document.removeEventListener('mouseup', up);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      localStorage.setItem(WIDTH_KEY, String(Math.round(latest)));
    };
    document.addEventListener('mousemove', move);
    document.addEventListener('mouseup', up);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  };

  if (!visible || !activeTask) return null;

  const runAction = async (id: string, prompt: string) => {
    setSending(id);
    try {
      await sendToAgent(sessionKey, prompt);
    } catch (e) {
      setLastError(String(e));
    } finally {
      setSending(null);
    }
  };

  const state = activity?.state ?? (agentPty ? 'working' : 'idle');

  return (
    <>
      {/* Maximized, the width and its handle are meaningless — it takes the body. */}
      {!maximized && <div className="resize-handle" onMouseDown={startDrag} />}
      <aside
        ref={paneRef}
        className={`agent-pane ${maximized ? 'maximized' : ''}`}
        style={maximized ? undefined : { width, minWidth: MIN_WIDTH }}
      >
        <div className="agent-pane-head">
          <span className={`pill-dot ${state}`} />
          <span className="console-target">{activeTask.short_id}</span>
          <span className="console-status">{statusText(activity, !!agentPty, starting)}</span>
          {/* Approving blind is not a state to discover later: while it is on, it is
              on screen, and one click ends it. */}
          {autoApprove && (
            <button
              className="console-trusted"
              onClick={() => setAutoApprove(false)}
              title="Every request from this session is approved automatically — click to start asking again"
            >
              <ShieldOff size={11} strokeWidth={2} />
              allowing all
            </button>
          )}
          <button
            className="dock-close"
            onClick={() => setOpen(false)}
            title="Hide the agent (Alt+A reopens)"
          >
            <X size={12} strokeWidth={2} />
          </button>
        </div>

        <div className="console-term" ref={termRef}>
          {!agentPty && (
            <span className="console-hint">
              {starting ? (
                <>
                  <Loader2 size={12} className="spin" /> Starting the agent…
                </>
              ) : (
                <button className="btn-secondary" onClick={start}>
                  <Play size={11} strokeWidth={2} style={{ marginRight: 5 }} />
                  Start an agent for {activeTask.short_id}
                </button>
              )}
            </span>
          )}
        </div>

        {/* Canned asks below the terminal — the conversation is the main thing. */}
        <div className="console-actions">
          {actionsFor(kind).map((a) => (
            <button
              key={a.id}
              className="console-action"
              title={a.title}
              disabled={!!sending}
              onClick={() =>
                runAction(a.id, a.build({
                  shortId: activeTask.short_id,
                  kind,
                  project: repos[0]?.project,
                  mrNumber: mrs[0] ? `!${mrs[0].remote_id}` : undefined,
                }))
              }
            >
              {sending === a.id && <Loader2 size={11} className="spin" />}
              {a.label}
            </button>
          ))}
        </div>
      </aside>
    </>
  );
}

/** One line describing the agent. No activity means nothing has been reported —
 *  said plainly rather than dressed up as idle. */
function statusText(a: AgentActivity | null, hasAgent: boolean, starting: boolean): string {
  if (starting) return 'starting…';
  if (!a) return hasAgent ? 'running' : 'no agent yet';
  const tool = a.tool ? (a.tool.detail ? `${a.tool.name}(${a.tool.detail})` : a.tool.name) : null;
  switch (a.state) {
    case 'waiting':
      return tool ? `waiting · ${tool}` : 'waiting on you';
    case 'working':
      return tool ?? 'working…';
    case 'idle':
      return a.last_message ? `idle · ${a.last_message}` : 'idle';
  }
}
