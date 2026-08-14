import type { ThemeName } from '../types/ipc';

/** Apply a color theme by toggling the `data-theme` attribute on <html>.
 *  The matching styles/themes/*.css block then drives every CSS variable. */
export function applyTheme(theme: ThemeName) {
  document.documentElement.setAttribute('data-theme', theme);
}

/**
 * Override the monospace family for the editor, tree and terminal.
 *
 * Everything CSS-driven reads `--font-mono`, so one variable covers them; xterm
 * takes a JS string instead and is updated by terminalHost's subscriber. An empty
 * value restores the stylesheet default rather than setting an empty family.
 */
export function applyFontFamily(family: string | undefined) {
  const r = document.documentElement.style;
  if (family?.trim()) r.setProperty('--font-mono', `'${family}', ui-monospace, monospace`);
  else r.removeProperty('--font-mono');
}

/** Rescale the whole UI by overriding the font-size token ramp on <html>. */
export function applyFontSize(px: number) {
  const r = document.documentElement.style;
  r.setProperty('--gl-font-size-xs', `${px - 2}px`);
  r.setProperty('--gl-font-size-sm', `${px}px`);
  r.setProperty('--gl-font-size-md', `${px + 1}px`);
  r.setProperty('--gl-font-size-lg', `${px + 3}px`);
  r.setProperty('--gl-font-size-xl', `${px + 5}px`);
}
