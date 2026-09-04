import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react';
import { ChevronUp, Loader2, Play, RefreshCw, Sparkles } from 'lucide-react';
import { offeredSkills } from '../shared/lib/skills';
import { providerCopy } from '../shared/lib/taskProvider';
import type { AgentActivity, AgentSkill, ProviderId, SessionKind } from '../shared/ipc/ipc';

/** The font the agent's terminal always uses, exempt from the config family. */
export const AGENT_FONT = "'Lilex', 'IBM Plex Mono', ui-monospace, monospace";

export interface AgentPanelProps {
  taskId: string;
  ptyId: string | null;
  kind: SessionKind;
  activity: AgentActivity | null;
  skills: AgentSkill[];
  skillsStale: boolean;
  /** Task sources set up; with several, create-task gets one row each. */
  sources: ProviderId[];
  autoApprove: boolean;
  /** The element the terminal host is re-parented into. */
  termRef: RefObject<HTMLDivElement>;
  /** A start already in flight outside the panel (the docked auto-start). */
  starting?: boolean;
  onStart: () => Promise<unknown>;
  onRunSkill: (skillId: string, args?: string) => Promise<unknown>;
  onReload: () => Promise<unknown>;
  onSetAutoApprove: (v: boolean) => void;
  /** Buttons at the right end of the head. */
  headActions?: ReactNode;
  /** The head is the window's title bar: it drags the window. */
  headIsTitleBar?: boolean;
}

/**
 * The agent surface: head, terminal, actions bar. Rendered docked as a column in
 * the main window and alone in the detached window; the container is the caller's.
 * The callbacks return promises so the panel can show what is in flight — errors
 * are the caller's to surface.
 */
export function AgentPanel(p: AgentPanelProps) {
  const [starting, setStarting] = useState(false);
  const [sending, setSending] = useState<string | null>(null);
  const [reloading, setReloading] = useState(false);
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

  const start = () => {
    if (starting) return;
    setStarting(true);
    p.onStart().catch(() => {}).finally(() => setStarting(false));
  };
  const runSkill = (skill: AgentSkill, args?: string) => {
    setSending(skill.id);
    p.onRunSkill(skill.id, args).catch(() => {}).finally(() => setSending(null));
  };
  const reload = () => {
    if (reloading) return;
    setReloading(true);
    p.onReload().catch(() => {}).finally(() => setReloading(false));
  };

  const offered = offeredSkills(p.skills, p.kind);
  const core = offered.filter((a) => !a.editable);
  const mine = offered.filter((a) => a.editable);

  const skillRow = (a: AgentSkill, src?: ProviderId) => (
    <button
      key={src ? `${a.id}:${src}` : a.id}
      className="ctx-menu-item"
      title={a.hint}
      onClick={() => {
        setMenuOpen(false);
        runSkill(a, src);
      }}
    >
      <Sparkles size={13} strokeWidth={1.75} />
      {src ? `File in ${providerCopy({ provider: src }).label}` : a.label}
    </button>
  );
  const skillRows = (list: AgentSkill[]) =>
    list.flatMap((a) =>
      a.name === 'create-task' && p.sources.length > 1
        ? p.sources.map((src) => skillRow(a, src))
        : [skillRow(a)],
    );

  const busyStarting = starting || !!p.starting;
  const state = p.activity?.state ?? (p.ptyId ? 'working' : 'idle');
  const drag = p.headIsTitleBar ? { 'data-tauri-drag-region': true } : {};

  return (
    <>
      <div className="agent-pane-head" {...drag}>
        <span className={`pill-dot ${state}`} />
        <span className="console-target" {...drag}>{p.taskId}</span>
        <span className="console-status" {...drag}>{statusText(p.activity, !!p.ptyId, busyStarting)}</span>
        {p.headActions}
      </div>

      <div className="console-term" ref={p.termRef}>
        {!p.ptyId && (
          <span className="console-hint">
            {busyStarting ? (
              <>
                <Loader2 size={12} className="spin" /> Starting the agent…
              </>
            ) : (
              <button className="btn-secondary" onClick={start}>
                <Play size={11} strokeWidth={2} style={{ marginRight: 5 }} />
                Start an agent for {p.taskId}
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
              {core.length > 0 && <div className="ctx-menu-label">Core</div>}
              {skillRows(core)}
              {mine.length > 0 && <div className="ctx-menu-label">User</div>}
              {skillRows(mine)}
            </div>
          )}
        </span>
        {/* A skill changed on disk; the running agent needs a restart to see it. */}
        {p.skillsStale && p.ptyId && (
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
          className={`console-toggle ${p.autoApprove ? 'on' : ''}`}
          role="switch"
          aria-checked={p.autoApprove}
          onClick={() => p.onSetAutoApprove(!p.autoApprove)}
          title={p.autoApprove
            ? 'Every request from this session is approved automatically — click to start asking again'
            : 'Approve every request from this session automatically'}
        >
          <span className="console-toggle-track"><span className="console-toggle-knob" /></span>
          Allow all
        </button>
      </div>
    </>
  );
}

/** One line describing the agent. */
export function statusText(a: AgentActivity | null, hasAgent: boolean, starting: boolean): string {
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
