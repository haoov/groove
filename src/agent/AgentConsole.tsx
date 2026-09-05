import { useEffect, useRef, useState } from 'react';
import { PictureInPicture2, X } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { shortcutLabel } from '../shared/lib/keybindings';
import { ensureAgentSession, reloadAgent, sendSkill } from '../shared/lib/agentSend';
import { SOURCE_IDS } from '../setup/sources';
import { focusHost } from '../shared/lib/terminalHost';
import { useAttachedHost } from '../shared/lib/useAttachedHost';
import { goToSessionById } from '../shared/lib/goToSession';
import { endSession } from '../shared/lib/endSession';
import { waitingCount } from '../shared/lib/agents';
import { AgentPanel } from './AgentPanel';
import { AgentsToggle } from './AgentsToggle';
import { useAgentRows } from './useAgentRows';

/**
 * The agent's REAL terminal, as a column on the right — not a re-implementation, so
 * there is no second input to keep in sync. Addresses the session its context names.
 * Detached, the column is not rendered at all: the agent window is the surface.
 */

const MIN_WIDTH = 320;
const MAX_WIDTH = 900;
const DEFAULT_WIDTH = 460;
const WIDTH_KEY = 'wb.agentPaneWidth';

export function AgentConsole() {
  const sessionKey = useSession((s) => s.id);
  const activeTask = useSession((s) => s.activeTask);
  const agentHint = useStore((s) => shortcutLabel(s.keymap, 'agent.console'));
  const ptySessions = useSession((s) => s.ptySessions);
  const kind = useSession((s) => s.kind);
  const autoApprove = useSession((s) => s.autoApprove);
  const setAutoApprove = useSession((s) => s.setAutoApprove);
  const skills = useStore((s) => s.skills);
  const skillsStale = useStore((s) => s.skillsStale);
  const open = useStore((s) => s.consoleOpen);
  const setOpen = useStore((s) => s.setConsoleOpen);
  const focusNonce = useStore((s) => s.consoleFocusNonce);
  const activity = useStore((s) =>
    activeTask ? s.agentActivity[activeTask.short_id] ?? null : null,
  );
  const setLastError = useStore((s) => s.setLastError);
  const maximized = useStore((s) => s.agentMaximized);
  const detached = useStore((s) => s.agentDetached);
  const setDetached = useStore((s) => s.setAgentDetached);
  const agentsOpen = useStore((s) => s.agentsSidebarOpen);
  const setAgentsOpen = useStore((s) => s.setAgentsSidebarOpen);
  const agentsHint = useStore((s) => shortcutLabel(s.keymap, 'agents.sidebar'));
  const agentsWidth = useStore((s) => s.agentsSidebarWidth);
  const setAgentsWidth = useStore((s) => s.setAgentsSidebarWidth);
  const agents = useAgentRows();
  // The list adds to the column, so the terminal keeps its width.
  const extra = agentsOpen ? agentsWidth : 0;

  const [starting, setStarting] = useState(false);

  // Where a filed task can go; the create-task rows of the Actions menu.
  const sources = useStore((s) => SOURCE_IDS.filter((id) => !!s.config?.[id]));
  const [width, setWidth] = useState(() => {
    const saved = Number(localStorage.getItem(WIDTH_KEY));
    return Number.isFinite(saved) && saved >= MIN_WIDTH ? saved : DEFAULT_WIDTH;
  });
  const termRef = useRef<HTMLDivElement>(null);
  const paneRef = useRef<HTMLElement>(null);

  const agentPty = ptySessions.find((p) => p.ptyType === 'agent')?.sessionId ?? null;
  const visible = !!activeTask && open && !detached;

  const start = () => {
    if (starting) return Promise.resolve();
    setStarting(true);
    return ensureAgentSession(sessionKey)
      .catch((e) => setLastError(String(e)))
      .finally(() => setStarting(false));
  };

  // Start an agent on the open TRANSITION only — not on every session switch, and
  // not in a loop after a failed start.
  const wasOpen = useRef(false);
  useEffect(() => {
    const justOpened = open && !wasOpen.current;
    wasOpen.current = open;
    if (!justOpened || !activeTask || agentPty) return;
    void start();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, activeTask, agentPty]);

  // Attach the terminal only while actually showing it.
  const holding = visible ? agentPty : null;
  useAttachedHost(holding, termRef, true);

  useEffect(() => {
    if (holding) focusHost(holding);
  }, [focusNonce, holding]);

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    // The rendered width, not the stored one — the pane may have been shrunk.
    const rendered = paneRef.current?.getBoundingClientRect().width;
    const startWidth = rendered === undefined ? width : rendered - extra;
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

  const report = (p: Promise<unknown>) => p.catch((e) => { setLastError(String(e)); throw e; });

  return (
    <>
      {/* Maximized, the width and its handle are meaningless — it takes the body. */}
      {!maximized && <div className="resize-handle" onMouseDown={startDrag} />}
      <aside
        ref={paneRef}
        className={`agent-pane ${maximized ? 'maximized' : ''}`}
        style={maximized ? undefined : { width: width + extra, minWidth: MIN_WIDTH + extra }}
      >
        <AgentPanel
          taskId={activeTask.short_id}
          ptyId={agentPty}
          kind={kind}
          activity={activity}
          skills={skills}
          skillsStale={skillsStale}
          sources={sources}
          autoApprove={autoApprove}
          termRef={termRef}
          starting={starting}
          onStart={start}
          onRunSkill={(id, args) => report(sendSkill(sessionKey, id, args))}
          onReload={() => report(reloadAgent(sessionKey))}
          onSetAutoApprove={setAutoApprove}
          agents={agents}
          agentsOpen={agentsOpen}
          agentsWidth={agentsWidth}
          onResizeAgents={setAgentsWidth}
          onGoToSession={(row) => goToSessionById(row.sessionId, { agent: true })}
          onCloseSession={(row) => void endSession(row.sessionId)}
          headActions={
            <>
              <AgentsToggle
                open={agentsOpen}
                count={waitingCount(agents)}
                hint={agentsHint}
                onClick={() => setAgentsOpen(!agentsOpen)}
              />
              <button
                className="pane-close"
                onClick={() => setDetached(true)}
                title="Pop the agent out into its own window"
              >
                <PictureInPicture2 size={12} strokeWidth={2} />
              </button>
              <button
                className="pane-close"
                onClick={() => setOpen(false)}
                title={`Hide the agent${agentHint ? ` (${agentHint} reopens)` : ''}`}
              >
                <X size={12} strokeWidth={2} />
              </button>
            </>
          }
        />
      </aside>
    </>
  );
}
