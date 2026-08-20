import { useEffect, useRef } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { call } from '../shared/ipc/client';
import { on, EV } from '../shared/ipc/events';

const cssVar = (n: string) => getComputedStyle(document.documentElement).getPropertyValue(n).trim() || undefined;
function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

/** A terminal attached to an already-running PTY (the forge-CLI sign-in shell).
 *  Unlike TerminalTab it does not start a session — it binds to a given sid. */
export function AuthTerminal({ sessionId }: { sessionId: string }) {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!host.current) return;
    const enc = new TextEncoder();
    const term = new XTerm({ fontFamily: cssVar('--font-mono') || 'monospace', fontSize: 12, cursorBlink: true,
      theme: { background: cssVar('--ctp-base'), foreground: cssVar('--ctp-text'), cursor: cssVar('--ctp-blue') } });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host.current);
    fit.fit();
    const offOut = on<{ session_id: string; b64: string }>(EV.ptyOutput, (p) => { if (p.session_id === sessionId) term.write(b64ToBytes(p.b64)); });
    const offExit = on<{ session_id: string }>(EV.ptyExit, (p) => { if (p.session_id === sessionId) term.write('\r\n\x1b[90m[shell exited]\x1b[0m\r\n'); });
    term.onData((d) => void call('write_pty', { sessionId, data: Array.from(enc.encode(d)) }));
    term.onResize(({ rows, cols }) => void call('resize_pty', { sessionId, rows, cols }));
    void call('resize_pty', { sessionId, rows: term.rows, cols: term.cols });
    term.focus();
    const ro = new ResizeObserver(() => fit.fit());
    ro.observe(host.current);
    return () => { ro.disconnect(); void offOut.then((f) => f()); void offExit.then((f) => f()); term.dispose(); };
  }, [sessionId]);
  return <div className="auth-term" ref={host} />;
}
