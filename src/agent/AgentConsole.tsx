import { useEffect, useRef, useState } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { call } from '../shared/ipc/client';
import { on, EV } from '../shared/ipc/events';
import type { AgentActivity, AgentState } from '../shared/ipc/generated';

const cssVar = (name: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim() || undefined;

function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

const STATE_LABEL: Record<AgentState, string> = { idle: 'Idle', working: 'Working', waiting: 'Waiting on you' };

/** The task's agent: Claude Code in a backend PTY, with a live activity state
 *  from the agent hooks and a prompt box that types straight into it. Keyed by
 *  session id at the mount site, so each session keeps its own console. */
export function AgentConsole({ sessionId, onCollapse }: { sessionId: string; onCollapse?: () => void }) {
  const host = useRef<HTMLDivElement>(null);
  const sid = useRef('');
  const [activity, setActivity] = useState<AgentActivity | null>(null);
  const [prompt, setPrompt] = useState('');

  useEffect(() => {
    if (!host.current) return;
    const enc = new TextEncoder();

    const term = new XTerm({
      fontFamily: cssVar('--font-mono') || 'monospace',
      fontSize: 12,
      cursorBlink: true,
      theme: {
        background: cssVar('--ctp-base'),
        foreground: cssVar('--ctp-text'),
        cursor: cssVar('--ctp-blue'),
        selectionBackground: cssVar('--ctp-surface2'),
        black: cssVar('--ctp-surface1'), brightBlack: cssVar('--ctp-surface2'),
        red: cssVar('--ctp-red'), brightRed: cssVar('--ctp-red'),
        green: cssVar('--ctp-green'), brightGreen: cssVar('--ctp-green'),
        yellow: cssVar('--ctp-yellow'), brightYellow: cssVar('--ctp-yellow'),
        blue: cssVar('--ctp-blue'), brightBlue: cssVar('--ctp-blue'),
        magenta: cssVar('--ctp-mauve'), brightMagenta: cssVar('--ctp-mauve'),
        cyan: cssVar('--ctp-teal'), brightCyan: cssVar('--ctp-teal'),
        white: cssVar('--ctp-subtext1'), brightWhite: cssVar('--ctp-text'),
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host.current);
    fit.fit();

    const offOut = on<{ session_id: string; b64: string }>(EV.ptyOutput, (p) => {
      if (p.session_id === sid.current) term.write(b64ToBytes(p.b64));
    });
    const offExit = on<{ session_id: string }>(EV.ptyExit, (p) => {
      if (p.session_id === sid.current) term.write('\r\n\x1b[90m[agent exited]\x1b[0m\r\n');
    });
    const offAct = on<AgentActivity>(EV.agentActivity, (a) => {
      if (a.task_id === sessionId) setActivity(a);
    });

    term.onData((d) => {
      if (sid.current) void call('write_pty', { sessionId: sid.current, data: Array.from(enc.encode(d)) });
    });
    term.onResize(({ rows, cols }) => {
      if (sid.current) void call('resize_pty', { sessionId: sid.current, rows, cols });
    });

    // Hydrate the current state; the map is in-memory, so a fresh backend has none.
    void call<AgentActivity[]>('get_agent_activity')
      .then((all) => { const mine = all.find((a) => a.task_id === sessionId); if (mine) setActivity(mine); })
      .catch(() => {});

    call<string>('start_agent_session', { taskId: sessionId })
      .then((id) => {
        sid.current = id;
        void call('resize_pty', { sessionId: id, rows: term.rows, cols: term.cols });
      })
      .catch((e) => term.write(`\r\n\x1b[31mfailed to start agent: ${e}\x1b[0m\r\n`));

    const ro = new ResizeObserver(() => fit.fit());
    ro.observe(host.current);

    return () => {
      ro.disconnect();
      void offOut.then((f) => f());
      void offExit.then((f) => f());
      void offAct.then((f) => f());
      if (sid.current) void call('stop_agent_session', { sessionId: sid.current });
      term.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  const sendPrompt = () => {
    const text = prompt.trim();
    if (!text || !sid.current) return;
    const enc = new TextEncoder();
    void call('write_pty', { sessionId: sid.current, data: Array.from(enc.encode(text + '\r')) });
    setPrompt('');
  };

  const state = activity?.state ?? 'idle';
  const detail = activity?.tool?.detail ?? activity?.tool?.name ?? '';

  return (
    <section className="agent">
      <div className="agent-h">
        <span className={`agent-dot s-${state}`} />
        <span className="agent-state">{STATE_LABEL[state]}</span>
        {detail && <span className="agent-detail">{detail}</span>}
        <span className="spring" />
        {onCollapse && <button className="agent-collapse" title="Hide agent" onClick={onCollapse}>⟩</button>}
      </div>
      <div className="agent-term" ref={host} />
      <div className="agent-prompt">
        <input
          placeholder="Send a prompt to the agent…"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') sendPrompt(); }}
        />
        <button disabled={!prompt.trim()} onClick={sendPrompt}>Send</button>
      </div>
    </section>
  );
}
