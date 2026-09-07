import type { ThemeName } from '../ipc/ipc';

/** Applies a colour theme through the `data-theme` attribute on <html>. */
export function applyTheme(theme: ThemeName) {
  document.documentElement.setAttribute('data-theme', theme);
}

/** Overrides `--font-mono`; xterm is updated by terminalHost's subscriber.
 *  An empty value restores the stylesheet default. */
export function applyFontFamily(family: string | undefined) {
  const r = document.documentElement.style;
  if (family?.trim()) r.setProperty('--font-mono', `'${family}', ui-monospace, monospace`);
  else r.removeProperty('--font-mono');
}

/** Rescales the UI by overriding the font-size token ramp on <html>. */
export function applyFontSize(px: number) {
  const r = document.documentElement.style;
  r.setProperty('--gl-font-size-xs', `${px - 2}px`);
  r.setProperty('--gl-font-size-sm', `${px}px`);
  r.setProperty('--gl-font-size-md', `${px + 1}px`);
  r.setProperty('--gl-font-size-lg', `${px + 3}px`);
  r.setProperty('--gl-font-size-xl', `${px + 5}px`);
}
