// The app's `invoke`: Tauri's, plus opt-in call timing — the frontend mirror of
// the backend's `RUST_LOG=timing=debug`. Flip it on from the devtools console:
//   localStorage.setItem('wb.ipcTiming', '1')
// Every IPC call then logs `name durationMs`, so call-count and latency
// regressions are observed instead of guessed. Off (the default) adds nothing
// to the hot path but one localStorage read at module load.

import { invoke as tauriInvoke, type InvokeArgs, type InvokeOptions } from '@tauri-apps/api/core';

let timing = false;
try { timing = localStorage.getItem('wb.ipcTiming') === '1'; } catch { /* SSR/tests */ }

export async function invoke<T>(cmd: string, args?: InvokeArgs, options?: InvokeOptions): Promise<T> {
  if (!timing) return tauriInvoke<T>(cmd, args, options);
  const t0 = performance.now();
  try {
    return await tauriInvoke<T>(cmd, args, options);
  } finally {
    console.debug(`[ipc] ${cmd} ${(performance.now() - t0).toFixed(1)}ms`);
  }
}
