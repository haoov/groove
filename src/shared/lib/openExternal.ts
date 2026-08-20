import { openUrl } from '@tauri-apps/plugin-opener';

/**
 * Open a URL in the system browser. Plain <a target="_blank"> doesn't work in a
 * Tauri webview (navigation is blocked), so links must route through the opener.
 */
export function openExternal(url: string | undefined | null) {
  if (!url) return;
  openUrl(url).catch((e) => console.error('openUrl failed:', e));
}
