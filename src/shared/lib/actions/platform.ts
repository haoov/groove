import { setPlatform, platform, type Platform } from '../pure/platform';

/**
 * Resolves the platform and marks it on `<html>` as `data-platform`.
 * Must settle before the app module graph loads: the store builds the default keymap on import.
 * Keep the `invoke` import dynamic: `isMac()` callers' tests run in plain node.
 */
export async function initPlatform(): Promise<Platform> {
  try {
    const { invoke } = await import('../../ipc/invoke');
    const os = await invoke<string>('platform');
    if (os === 'macos' || os === 'linux' || os === 'windows') setPlatform(os);
  } catch {
    // Keep the default.
  }
  document.documentElement.setAttribute('data-platform', platform());
  return platform();
}
