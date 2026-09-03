import { useEffect, useRef, useState } from 'react';
import { ChevronUp, Loader2, Play, RefreshCw, Sparkles, X } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { shortcutLabel } from '../shared/lib/keybindings';
import { ensureAgentSession, reloadAgent, sendSkill } from '../shared/lib/agentSend';
import { offeredSkills } from '../shared/lib/skills';
import { SOURCE_IDS } from '../setup/sources';
import { providerCopy } from '../shared/lib/taskProvider';
import { focusHost } from '../shared/lib/terminalHost';
import { useAttachedHost } from '../shared/lib/useAttachedHost';
import type { AgentActivity, AgentSkill, ProviderId } from '../shared/ipc/ipc';

/**
 * The agent's REAL terminal, as a column on the right — not a re-implementation, so
 * there is no second input to keep in sync. Addresses the session its context names.
 */

const MIN_WIDTH = 320;
const MAX_WIDTH = 900;
const DEFAULT_WIDTH = 460;
const WIDTH_KEY = 'wb.agentPaneWidth';

const AGENT_FONT = "'Lilex', 'IBM Plex Mono', ui-monospace, monospace";

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

  const [starting, setStarting] = useState(false);
  const [sending, setSending] = useState<string | null>(null);
  const [reloading, setReloading] = useState(false);

  // Where a filed task can go; the create-task rows of the Actions menu.
  const sources = useStore((s) => SOURCE_IDS.filter((id) => !!s.config?.[id]));
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    if (!menuOpen) return;
    const onDown = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node)) setMenuOpen(false);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setMenuOpen(false); };
    window.addEventListener('mousedown', onDown, true);
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('mousedown', onDown, true);
      window.removeEventListener('keydown', onKey, true);
    };
  }, [menuOpen]);
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

  // Start an agent on the open TRANSITION only — not on every session switch, and
  // not in a loop after a failed start.
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
  useAttachedHost(holding, termRef, AGENT_FONT);

  useEffect(() => {
    if (holding) focusHost(holding);
  }, [focusNonce, holding]);

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    // The rendered width, not the stored one — the pane may have been shrunk.
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

  const runSkill = async (skill: AgentSkill, args?: string) => {
    setSending(skill.id);
    try {
      await sendSkill(sessionKey, skill.id, args);
    } catch (e) {
      setLastError(String(e));
    } finally {
      setSending(null);
    }
  };

  const offered = offeredSkills(skills, kind);
  const core = offered.filter((a) => !a.editable);
  const mine = offered.filter((a) => a.editable);

  // With several sources configured, create-task gets one row each; the pick is
  // the skill's argument.
  const skillRow = (a: AgentSkill, src?: ProviderId) => (
    <button
      key={src ? `${a.id}:${src}` : a.id}
      className="ctx-menu-item"
      title={a.hint}
      onClick={() => {
        setMenuOpen(false);
        void runSkill(a, src);
      }}
    >
      <Sparkles size={13} strokeWidth={1.75} />
      {src ? `File in ${providerCopy({ provider: src }).label}` : a.label}
    </button>
  );
  const skillRows = (list: AgentSkill[]) =>
    list.flatMap((a) =>
      a.name === 'create-task' && sources.length > 1
        ? sources.map((src) => skillRow(a, src))
        : [skillRow(a)],
    );

  const reload = () => {
    if (reloading) return;
    setReloading(true);
    reloadAgent(sessionKey)
      .catch((e) => setLastError(String(e)))
      .finally(() => setReloading(false));
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
          <button
            className="dock-close"
            onClick={() => setOpen(false)}
            title={`Hide the agent${agentHint ? ` (${agentHint} reopens)` : ''}`}
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

        {/* The skills, behind one menu. */}
        <div className="console-actions">
          <span className="actions-menu" ref={menuRef}>
            <button
              className="console-action"
              title="What you can ask this agent for"
              disabled={!!sending || offered.length === 0}
              onClick={() => setMenuOpen((v) => !v)}
            >
              {sending ? <Loader2 size={11} className="spin" /> : <Sparkles size={11} strokeWidth={2} />}
              Actions
              <ChevronUp size={11} strokeWidth={2} />
            </button>
            {/* Opens UPWARD: the bar is the console's bottom edge. */}
            {menuOpen && (
              <div className="ctx-menu actions-menu-panel">
                {/* One heading per group. */}
                {core.length > 0 && <div className="ctx-menu-label">Core</div>}
                {skillRows(core)}
                {mine.length > 0 && <div className="ctx-menu-label">User</div>}
                {skillRows(mine)}
              </div>
            )}
          </span>
          {/* A skill changed on disk; the running agent needs a restart to see it. */}
          {skillsStale && agentPty && (
            <button
              className="console-action"
              disabled={reloading}
              onClick={reload}
              title="Restart the agent so it loads the skills — the conversation is resumed"
            >
              {reloading ? <Loader2 size={11} className="spin" /> : <RefreshCw size={11} strokeWidth={2} />}
              Reload skills
            </button>
          )}
          {/* Auto-approve — warm while on. */}
          <button
            className={`console-toggle ${autoApprove ? 'on' : ''}`}
            role="switch"
            aria-checked={autoApprove}
            onClick={() => setAutoApprove(!autoApprove)}
            title={autoApprove
              ? 'Every request from this session is approved automatically — click to start asking again'
              : 'Approve every request from this session automatically'}
          >
            <span className="console-toggle-track"><span className="console-toggle-knob" /></span>
            Allow all
          </button>
        </div>
      </aside>
    </>
  );
}

/** One line describing the agent. */
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
