import { openUrl } from '@tauri-apps/plugin-opener';

/** Opens a URL in the system browser. `<a target="_blank">` does not navigate in a Tauri webview. */
export function openExternal(url: string | undefined | null) {
  if (!url) return;
  openUrl(url).catch((e) => console.error('openUrl failed:', e));
}
