import { useEffect, useRef } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useStore } from '../shared/store';

/**
 * Measures time on the focused task. A tick counts only while the window has focus
 * and there is recent input or a busy agent. Accumulates locally; nothing writes to the task source.
 */

const TICK_MS = 30_000;
/** No interaction for this long and the clock stops — unless the agent is busy. */
const IDLE_MS = 5 * 60_000;

export function useTaskTimer() {
  const activeShortId = useStore((s) =>
    s.activeSessionId ? s.sessions[s.activeSessionId]?.task?.short_id ?? null : null,
  );
  // Seeded on mount.
  const lastInputRef = useRef(0);
  const focusedRef = useRef(true);

  // Any interaction anywhere in the app counts as "still here".
  useEffect(() => {
    lastInputRef.current = Date.now();
    const touch = () => { lastInputRef.current = Date.now(); };
    const events = ['keydown', 'mousedown', 'wheel', 'mousemove'] as const;
    for (const e of events) window.addEventListener(e, touch, { passive: true });
    return () => { for (const e of events) window.removeEventListener(e, touch); };
  }, []);

  // Tauri's focus signal; `document.hasFocus()` misses an OS-level focus loss.
  useEffect(() => {
    const w = getCurrentWindow();
    w.isFocused().then((f) => { focusedRef.current = f; }).catch(() => {});
    const unlisten = w.onFocusChanged(({ payload }) => {
      focusedRef.current = payload;
      // A focus return counts as input.
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
      // Best-effort; a dropped tick is lost.
      invoke('add_task_time', { taskId: activeShortId, seconds }).catch(() => {});
    }, TICK_MS);
    return () => clearInterval(id);
  }, [activeShortId]);
}
