import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, Play, X } from 'lucide-react';
import { useSession, useStore } from '../shared/store';
import { focusHost } from './terminalHost';
import { useAttachedHost } from './useAttachedHost';

/**
 * A terminal on Home.
 *
 * In a workspace, terminals are panes in the bottom dock — they belong to a task and
 * sit beside its files. Home has no panes, but it is where you land to look at the
 * queue, and needing a shell there meant opening a task you did not want to work on.
 *
 * So this is the terminal equivalent of the agent console: app-level, addressing the
 * session named by the context (the desk on Home), and docked at the bottom because a
 * shell reads short and wide. Only mounted outside a workspace, so there is never a
 * second terminal surface competing with the panes.
 */

const MIN_HEIGHT = 120;
const MAX_HEIGHT = 720;
const DEFAULT_HEIGHT = 260;
const HEIGHT_KEY = 'wb.homeTerminalHeight';

export function TerminalConsole() {
  const open = useStore((s) => s.terminalConsoleOpen);
  const setOpen = useStore((s) => s.setTerminalConsoleOpen);
  const focusReq = useStore((s) => s.terminalFocusReq);
  const setLastError = useStore((s) => s.setLastError);

  const activeTask = useSession((s) => s.activeTask);
  const ptySessions = useSession((s) => s.ptySessions);

  const [starting, setStarting] = useState(false);
  const [height, setHeight] = useState(() => {
    const saved = Number(localStorage.getItem(HEIGHT_KEY));
    return Number.isFinite(saved) && saved >= MIN_HEIGHT ? saved : DEFAULT_HEIGHT;
  });
  const termRef = useRef<HTMLDivElement>(null);

  const pty = ptySessions.find((p) => p.ptyType === 'terminal')?.sessionId ?? null;
  useAttachedHost(open ? pty : null, termRef);

  const start = () => {
    if (starting || !activeTask) return;
    setStarting(true);
    // No worktree: the desk has none, so the backend falls back to the worktree
    // root — the same cwd an agent gets.
    invoke<string>('start_terminal_session', { taskId: activeTask.short_id, worktreePath: null })
      .catch((e) => setLastError(String(e)))
      .finally(() => setStarting(false));
  };

  // Opening IS the request to start one, but only on the open transition: with it
  // already open, switching sessions must not spawn a shell in each.
  const wasOpen = useRef(false);
  useEffect(() => {
    const justOpened = open && !wasOpen.current;
    wasOpen.current = open;
    if (!justOpened || !activeTask || pty) return;
    start();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, activeTask, pty]);

  useEffect(() => {
    if (open && pty && focusReq) focusHost(pty);
  }, [open, pty, focusReq]);

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = height;
    let latest = startHeight;
    const move = (ev: MouseEvent) => {
      // Dragging UP grows it: the handle is on the dock's top edge.
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

  if (!open || !activeTask) return null;

  return (
    <div className="home-terminal" style={{ height }} data-dock="terminal">
      <div className="resize-handle-h" onMouseDown={startDrag} />
      <div className="home-terminal-head">
        <span className="console-target">terminal</span>
        <span className="console-status">{activeTask.short_id}</span>
        <button
          className="dock-close"
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
