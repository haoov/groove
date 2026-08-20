import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useStore } from '../shared/store';

/**
 * Measure time spent on the focused task.
 *
 * Deliberately strict: a tick only counts while the app window has focus AND
 * there is evidence of work — you interacted recently, or that task's agent is
 * mid-turn. So watching an agent grind through a refactor counts (it is work on
 * the task), but leaving the app open over lunch does not.
 *
 * Nothing here writes to Notion. It accumulates locally; the overview shows what
 * was measured and you log a figure you agree with (see hours.rs).
 */

const TICK_MS = 30_000;
/** No interaction for this long and the clock stops — unless the agent is busy. */
const IDLE_MS = 5 * 60_000;

export function useTaskTimer() {
  const activeShortId = useStore((s) =>
    s.activeSessionId ? s.sessions[s.activeSessionId]?.task?.short_id ?? null : null,
  );
  const lastInputRef = useRef(Date.now());
  const focusedRef = useRef(true);

  // Any interaction anywhere in the app counts as "still here".
  useEffect(() => {
    const touch = () => { lastInputRef.current = Date.now(); };
    const events = ['keydown', 'mousedown', 'wheel', 'mousemove'] as const;
    for (const e of events) window.addEventListener(e, touch, { passive: true });
    return () => { for (const e of events) window.removeEventListener(e, touch); };
  }, []);

  // Window focus is the hard gate. `document.hasFocus()` misses the case where
  // the OS window lost focus but the page kept it, so use Tauri's own signal.
  useEffect(() => {
    const w = getCurrentWindow();
    w.isFocused().then((f) => { focusedRef.current = f; }).catch(() => {});
    const unlisten = w.onFocusChanged(({ payload }) => {
      focusedRef.current = payload;
      // Returning to the window is itself a sign of presence.
      if (payload) lastInputRef.current = Date.now();
    });
    return () => { unlisten.then((f) => f()).catch(() => {}); };
  }, []);

  useEffect(() => {
    if (!activeShortId) return;
    const seconds = Math.round(TICK_MS / 1000);
    const id = window.setInterval(() => {
      if (!focusedRef.current) return;
      const idle = Date.now() - lastInputRef.current > IDLE_MS;
      const agentBusy = useStore.getState().agentActivity[activeShortId]?.state === 'working';
      if (idle && !agentBusy) return;
      // Best-effort: a dropped tick costs 30s of credit, never correctness.
      invoke('add_task_time', { taskId: activeShortId, seconds }).catch(() => {});
    }, TICK_MS);
    return () => clearInterval(id);
  }, [activeShortId]);
}
