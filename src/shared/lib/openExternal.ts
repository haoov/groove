import { openUrl } from '@tauri-apps/plugin-opener';

/** Open a URL in the user's browser (GitLab MR/CI pages, etc.). Best-effort. */
export function openExternal(url: string | null | undefined): void {
  if (url) void openUrl(url).catch((e) => console.warn('open external failed', e));
}
