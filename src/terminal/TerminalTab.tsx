import { useEffect, useRef } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { call } from '../shared/ipc/client';
import { on, EV } from '../shared/ipc/events';
import { useStore } from '../shared/store';
import { monoSize } from '../shared/lib/ui';

const cssVar = (name: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim() || undefined;

function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

/** A real terminal: xterm.js over a backend PTY. Output arrives base64 on
 *  `pty_output`; keystrokes go back as bytes via `write_pty`. Kept mounted while
 *  the tab is open so the session survives tab switches. */
export function TerminalTab({
  sessionId, tabId, taskId, cwd,
}: {
  sessionId: string; tabId: string; taskId: string; cwd: string | undefined;
}) {
  const host = useRef<HTMLDivElement>(null);
  const patchTab = useStore((s) => s.patchTab);

  useEffect(() => {
    if (!host.current) return;
    const enc = new TextEncoder();
    const sid = { current: '' };

    const term = new XTerm({
      fontFamily: cssVar('--font-mono') || 'monospace',
      fontSize: monoSize(),
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
      if (p.session_id === sid.current) term.write('\r\n\x1b[90m[process exited]\x1b[0m\r\n');
    });

    term.onData((d) => {
      if (sid.current) void call('write_pty', { sessionId: sid.current, data: Array.from(enc.encode(d)) });
    });
    term.onResize(({ rows, cols }) => {
      if (sid.current) void call('resize_pty', { sessionId: sid.current, rows, cols });
    });

    call<string>('start_terminal_session', { taskId, worktreePath: cwd })
      .then((id) => {
        sid.current = id;
        patchTab(sessionId, tabId, { ptySessionId: id });
        void call('resize_pty', { sessionId: id, rows: term.rows, cols: term.cols });
        term.focus();
      })
      .catch((e) => term.write(`\r\n\x1b[31mfailed to start shell: ${e}\x1b[0m\r\n`));

    const ro = new ResizeObserver(() => fit.fit());
    ro.observe(host.current);

    return () => {
      ro.disconnect();
      void offOut.then((f) => f());
      void offExit.then((f) => f());
      if (sid.current) void call('stop_agent_session', { sessionId: sid.current });
      term.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return <div className="term-host" ref={host} />;
}
