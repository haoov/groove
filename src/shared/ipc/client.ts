// The one place the frontend talks to the backend. A thin typed wrapper over
// Tauri's `invoke`: every command goes through `call`, so error shape and
// logging live in one spot. Command argument/return types come from the
// generated bindings (./generated) — never hand-mirrored.

import { invoke } from '@tauri-apps/api/core';

/**
 * Invoke a backend command. The backend returns `Result<T, String>`, so a
 * rejection is always a plain message string — surface it as an `Error`.
 */
export async function call<T = void>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw e instanceof Error ? e : new Error(typeof e === 'string' ? e : String(e));
  }
}

export type * from './generated';
