import { useEffect, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { RotateCcw } from 'lucide-react';
import { useStore, useSession, type EditorTab } from '../shared/store';
import { fitAndSync, focusHost } from '../shared/lib/terminalHost';
import { useAttachedHost } from '../shared/lib/useAttachedHost';

/**
 * Body of a terminal tab. The xterm lives in the module-level host registry —
 * this component only re-parents the host's element into its container (and
 * detaches on unmount), so the terminal survives tab switches, pane moves, and
 * tab close/reopen.
 *
 * Agents have no tab: they live in the console (components/AgentConsole).
 */
export function PtyTabBody({
  tab, paneId, isActive,
}: {
  tab: EditorTab;
  paneId: string;
  isActive: boolean;
}) {
  const ptySessions = useSession((s) => s.ptySessions);
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const setTabPty = useSession((s) => s.setTabPty);
  const reportError = useStore((s) => s.setLastError);
  const terminalFocusReq = useStore((s) => s.terminalFocusReq);

  const sessionId =
    tab.ptySessionId && ptySessions.some((p) => p.sessionId === tab.ptySessionId)
      ? tab.ptySessionId
      : null;

  // Distinguish "never started" (auto-start) from "session ended" (offer restart).
  const hadSessionRef = useRef(false);
  if (sessionId) hadSessionRef.current = true;
  const ended = !sessionId && hadSessionRef.current;

  const containerRef = useRef<HTMLDivElement>(null);
  const [starting, setStarting] = useState(false);

  const startSession = async () => {
    if (!activeTask || starting) return;
    setStarting(true);
    try {
      const wt = activeWorktrees[0];
      const sid = await invoke<string>('start_terminal_session', {
        taskId: activeTask.short_id,
        worktreePath: wt?.path ?? null,
      });
      setTabPty(paneId, tab.id, sid);
      hadSessionRef.current = true;
    } catch (e) {
      reportError(String(e));
    } finally {
      setStarting(false);
    }
  };

  // Auto-start on first mount (opening the tab IS the request to start).
  const autoStartedRef = useRef(false);
  useEffect(() => {
    if (!sessionId && !ended && activeTask && !autoStartedRef.current) {
      autoStartedRef.current = true;
      startSession();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId, ended, activeTask]);

  useAttachedHost(sessionId, containerRef);

  // A keybinding asked the terminal to take focus.
  const lastFocusNonce = useRef(0);
  useEffect(() => {
    if (!terminalFocusReq || !isActive || !sessionId) return;
    if (terminalFocusReq === lastFocusNonce.current) return;
    lastFocusNonce.current = terminalFocusReq;
    focusHost(sessionId);
  }, [terminalFocusReq, isActive, sessionId]);

  // Refit when this tab becomes visible again (display:none → flex): it measured
  // 0×0 while hidden, so its size is whatever it had when it was last shown.
  useEffect(() => {
    if (!isActive || !sessionId) return;
    const id = requestAnimationFrame(() => fitAndSync(sessionId));
    return () => cancelAnimationFrame(id);
  }, [isActive, sessionId]);

  if (!sessionId) {
    return (
      <div className="pty-tab-body" data-dock="terminal">
        <div className="agent-empty">
          {ended ? (
            <div className="pty-ended">
              <span className="agent-empty-hint">Terminal session ended</span>
              <button className="btn-secondary" onClick={startSession} disabled={starting}>
                <RotateCcw size={12} strokeWidth={1.75} style={{ marginRight: 5 }} />
                {starting ? 'Restarting…' : 'Restart'}
              </button>
            </div>
          ) : (
            <span className="agent-empty-hint">
              {activeTask ? 'Starting terminal…' : 'No active task'}
            </span>
          )}
        </div>
      </div>
    );
  }

  return <div className="pty-tab-body" data-dock="terminal" ref={containerRef} />;
}
