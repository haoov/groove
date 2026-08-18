import { invoke } from '@tauri-apps/api/core';

/**
 * Log a PTY hop into the backend's trace file.
 *
 * Free when tracing is off: the backend's marker file decides, and the answer is
 * asked for once. Without this the webview's two hops are invisible, and a
 * character that arrives twice cannot be blamed on the shell, the event delivery
 * or the renderer — which is what made the doubled-letters bug survive three
 * fixes aimed at the wrong hop.
 */
let enabled: boolean | null = null;

export function tracePty(tag: 'js>>' | 'js<<', sessionId: string, data: ArrayLike<number>) {
  if (enabled === false) return;
  if (enabled === null) {
    enabled = false; // until the answer lands, so this asks once
    invoke<boolean>('pty_trace_on')
      .then((on) => { enabled = on; })
      .catch(() => { enabled = false; });
    return;
  }
  invoke('trace_pty', { tag, sessionId, data: Array.from(data) }).catch(() => { enabled = false; });
}
