import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '../shared/ipc/invoke';
import { Loader2, Play, X } from 'lucide-react';
import { useStore } from '../shared/store';
import { focusHost } from '../shared/lib/terminalHost';
import { useAttachedHost } from '../shared/lib/useAttachedHost';
import { EVENT } from '../shared/ipc/events';
import type { PtyExitEvent } from '../shared/ipc/ipc';

/**
 * The scratch terminal on Home. Session-less: it owns its PTY under the synthetic id
 * "__scratch__", and the backend reaps the row on exit.
 */

const SCRATCH_TASK_ID = '__scratch__';

const MIN_HEIGHT = 120;
const MAX_HEIGHT = 720;
const DEFAULT_HEIGHT = 260;
const HEIGHT_KEY = 'wb.homeTerminalHeight';

// Module-level: the PTY survives the dock unmounting.
let scratchPty: string | null = null;

export function TerminalConsole() {
  const open = useStore((s) => s.terminalConsoleOpen);
  const setOpen = useStore((s) => s.setTerminalConsoleOpen);
  const focusReq = useStore((s) => s.terminalFocusReq);
  const setLastError = useStore((s) => s.setLastError);

  const [pty, setPty] = useState<string | null>(scratchPty);
  const [starting, setStarting] = useState(false);
  const [height, setHeight] = useState(() => {
    const saved = Number(localStorage.getItem(HEIGHT_KEY));
    return Number.isFinite(saved) && saved >= MIN_HEIGHT ? saved : DEFAULT_HEIGHT;
  });
  const termRef = useRef<HTMLDivElement>(null);

  useAttachedHost(open ? pty : null, termRef);

  const start = () => {
    if (starting) return;
    setStarting(true);
    // No worktree: the backend uses the worktree root as cwd.
    invoke<string>('start_terminal_session', { taskId: SCRATCH_TASK_ID, worktreePath: null })
      .then((id) => { scratchPty = id; setPty(id); })
      .catch((e) => setLastError(String(e)))
      .finally(() => setStarting(false));
  };

  // Start a shell on the open transition only.
  const wasOpen = useRef(false);
  useEffect(() => {
    const justOpened = open && !wasOpen.current;
    wasOpen.current = open;
    if (!justOpened || pty) return;
    start();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, pty]);

  // A shell exit clears the slot.
  useEffect(() => {
    let cancelled = false;
    const un = listen<PtyExitEvent>(EVENT.PTY_EXIT, ({ payload }) => {
      if (payload.session_id === scratchPty) {
        scratchPty = null;
        if (!cancelled) setPty(null);
      }
    });
    return () => { cancelled = true; un.then((f) => f()); };
  }, []);

  useEffect(() => {
    if (open && pty && focusReq) focusHost(pty);
  }, [open, pty, focusReq]);

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = height;
    let latest = startHeight;
    const move = (ev: MouseEvent) => {
      // Dragging up grows it.
      latest = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, startHeight + (startY - ev.clientY)));
      setHeight(latest);
    };
    const up = () => {
      document.removeEventListener('mousemove', move);
      document.removeEventListener('mouseup', up);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      localStorage.setItem(HEIGHT_KEY, String(Math.round(latest)));
    };
    document.addEventListener('mousemove', move);
    document.addEventListener('mouseup', up);
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
  };

  if (!open) return null;

  return (
    <div className="home-terminal" style={{ height }} data-dock="terminal">
      <div className="resize-handle-h" onMouseDown={startDrag} />
      <div className="home-terminal-head">
        <span className="console-target">terminal</span>
        <span className="console-status">scratch</span>
        <button
          className="pane-close"
          onClick={() => setOpen(false)}
          title="Hide the terminal (the terminal shortcut reopens it)"
        >
          <X size={12} strokeWidth={2} />
        </button>
      </div>
      <div className="console-term" ref={termRef}>
        {!pty && (
          <span className="console-hint">
            {starting ? (
              <><Loader2 size={12} className="spin" /> Starting a shell…</>
            ) : (
              <button className="btn-secondary" onClick={start}>
                <Play size={11} strokeWidth={2} style={{ marginRight: 5 }} />
                Start a shell
              </button>
            )}
          </span>
        )}
      </div>
    </div>
  );
}
