import { useEffect, useRef, useState } from 'react';
import { ChevronUp, Loader2, Play, RefreshCw, Sparkles, X } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { shortcutLabel } from '../shared/lib/keybindings';
import { ensureAgentSession, reloadAgent, sendSkill } from '../shared/lib/agentSend';
import { SOURCE_IDS } from '../setup/sources';
import { providerCopy } from '../shared/lib/taskProvider';
import { focusHost } from '../shared/lib/terminalHost';
import { useAttachedHost } from '../shared/lib/useAttachedHost';
import type { AgentActivity, AgentSkill, ProviderId } from '../shared/ipc/ipc';

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
 * workspace (App.tsx mounts it there only).
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
  useAttachedHost(holding, termRef, AGENT_FONT);

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

  const offered = skills.filter(
    (a) => !a.hidden && (a.kinds.length === 0 || a.kinds.includes(kind)),
  );
  const core = offered.filter((a) => !a.editable);
  const mine = offered.filter((a) => a.editable);

  // Filing a task needs a source; with several configured each one is its own
  // row and the pick rides along as the skill's argument. One source needs no
  // row of its own — the agent infers it.
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

        {/* The skills below the terminal — the conversation is the main thing,
            and all of them behind one menu so the bar stays a single line however
            many skills the user has written.
            An empty `kinds` offers the skill everywhere, which is what a user
            skill gets when they name no kind. */}
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
                {skillRows(core)}
                {/* The user's own sit below the line — the grouping is what says
                    whose a skill is, so a row needs no badge of its own. */}
                {core.length > 0 && mine.length > 0 && <div className="ctx-menu-sep" />}
                {skillRows(mine)}
              </div>
            )}
          </span>
          {/* A skill changed on disk. `--plugin-dir` is read at launch, so the
              running agent cannot see it — and the skills list deliberately does
              not refresh until it can, or the menu would offer a slash command
              the agent answers with "unknown command". */}
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
          {/* Auto-approve, as a real switch — warm while on, so approving blind is
              never a hidden state. Sits apart from the canned asks. */}
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
