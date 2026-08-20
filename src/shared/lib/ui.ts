import type { UiConfig } from '../ipc/generated';

export const THEMES = ['latte', 'frappe', 'onelight', 'onedark'] as const;
export type ThemeName = (typeof THEMES)[number];

/** Apply the saved UI preferences to the document: theme via `data-theme`, and
 *  the mono family + size as CSS vars the editor, tree, and terminals read. */
export function applyUiConfig(ui: UiConfig | undefined) {
  const root = document.documentElement;
  root.dataset.theme = ui?.theme || 'latte';
  if (ui?.font_family) {
    root.style.setProperty('--font-mono', `'${ui.font_family}', 'Lilex', 'IBM Plex Mono', ui-monospace, monospace`);
  }
  if (ui?.font_size) root.style.setProperty('--mono-size', `${ui.font_size}px`);
}

/** Editor/terminal font size in px, from the config var, with a sane default. */
export function monoSize(): number {
  const v = getComputedStyle(document.documentElement).getPropertyValue('--mono-size').trim();
  const n = parseInt(v, 10);
  return Number.isFinite(n) && n > 0 ? n : 12;
}
