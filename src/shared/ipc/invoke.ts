// The app's `invoke`: Tauri's, plus opt-in call timing.
// Enable with `localStorage.setItem('wb.ipcTiming', '1')`; every call then logs `name durationMs`.

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
