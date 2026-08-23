export type Platform = 'macos' | 'linux' | 'windows';

let current: Platform = 'linux';

export function platform(): Platform {
  return current;
}

export function isMac(): boolean {
  return current === 'macos';
}

/** Tests only; production resolves through initPlatform(). */
export function setPlatform(p: Platform): void {
  current = p;
}

/**
 * Resolve the platform and mark it on `<html>` as `data-platform`.
 *
 * Must settle before the app module graph loads: the store builds the default keymap
 * on import. `invoke` is imported dynamically to keep the Tauri API out of `isMac()`
 * callers, whose tests run in plain node.
 */
export async function initPlatform(): Promise<Platform> {
  try {
    const { invoke } = await import('../ipc/invoke');
    const os = await invoke<string>('platform');
    if (os === 'macos' || os === 'linux' || os === 'windows') current = os;
  } catch {
    // Keep the default.
  }
  document.documentElement.setAttribute('data-platform', current);
  return current;
}
