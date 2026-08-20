import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { invoke } from '@tauri-apps/api/core';
import { useStore } from '../store';
import { DEFAULT_FONT_SIZE } from '../ipc/ipc';
import { registerPtyHandler, unregisterPtyHandler, bytesToB64 } from './ptyRegistry';
import '@xterm/xterm/css/xterm.css';

/**
 * Terminal ownership lives OUTSIDE React: xterm cannot be re-`open()`ed into a
 * new element and its scrollback lives in the Terminal object, so hosts persist
 * for the PTY session's lifetime. Components only re-parent `el` into their
 * container on mount and detach it on unmount — moving a PTY tab between panes,
 * hiding it, or closing its tab never loses the terminal or its output (the PTY
 * handler writes into the Terminal even while no tab shows it).
 */
export interface TermHost {
  term: Terminal;
  fit: FitAddon;
  el: HTMLDivElement;
}

const hosts = new Map<string, TermHost>();

/** Resolve the active theme's palette (CSS custom properties) into a concrete
 *  xterm theme object — xterm can't read CSS variables. */
function xtermThemeFromCss() {
  const cs = getComputedStyle(document.documentElement);
  const v = (name: string) => cs.getPropertyValue(name).trim();
  return {
    background: v('--term-bg'),
    foreground: v('--term-fg'),
    cursor: v('--term-cursor'),
    cursorAccent: v('--term-bg'),
    selectionBackground: v('--term-selection'),
    black: v('--term-black'),       brightBlack: v('--term-bright-black'),
    red: v('--ctp-red'),            brightRed: v('--ctp-red'),
    green: v('--ctp-green'),        brightGreen: v('--ctp-green'),
    yellow: v('--ctp-yellow'),      brightYellow: v('--ctp-yellow'),
    blue: v('--ctp-blue'),          brightBlue: v('--ctp-blue'),
    magenta: v('--ctp-mauve'),      brightMagenta: v('--ctp-mauve'),
    cyan: v('--ctp-teal'),          brightCyan: v('--ctp-teal'),
    white: v('--term-white'),       brightWhite: v('--term-bright-white'),
  };
}

/** Quiet period after the last selection change before copy-on-select fires. */
const SELECTION_COPY_MS = 180;

/** Terminal reads a touch larger than the editor for glyph legibility. */
const termFontSize = () => (useStore.getState().config?.ui.font_size ?? DEFAULT_FONT_SIZE) + 1;

/** The configured family, with the same fallbacks as `--font-mono`. xterm cannot
 *  read CSS variables, so this mirrors the token in JS. */
const termFontFamily = () => {
  const configured = useStore.getState().config?.ui.font_family?.trim();
  const stack = `'FiraCode Nerd Font Mono', ui-monospace, monospace`;
  return configured ? `'${configured}', ${stack}` : `'JetBrainsMono Nerd Font Mono', ${stack}`;
};

/**
 * Clipboard for the terminals, through the backend.
 *
 * Not `navigator.clipboard` and not `execCommand('copy')`: the first is absent in
 * WebKitGTK unless the origin is a secure context, and the second needs a real DOM
 * selection, which a terminal never has — xterm draws its own. Both failed
 * silently. The backend shells out to the OS clipboard tool instead
 * (src-tauri/src/clipboard.rs), which is verifiable and reports errors.
 */
async function copyText(text: string): Promise<void> {
  await invoke('copy_to_clipboard', { text });
}

/**
 * Copy and paste inside a terminal.
 *
 * Ctrl+Shift+C / Ctrl+Shift+V, not the bare chords: Ctrl+C is SIGINT and Ctrl+V is
 * a literal keystroke the program may want.
 *
 * Selection needs no modifier. Claude Code turns on bracketed paste (`?2004h`) but
 * NOT mouse tracking — verified by capturing its output in a pty — so xterm never
 * forwards mousedown to the program and an ordinary drag selects, in the agent
 * terminal exactly as in the shell one.
 *
 * A finished selection is also copied automatically, the way most terminals behave:
 * a keystroke that silently does nothing is what made this hard to use at all.
 */
function attachClipboard(term: Terminal) {
  term.attachCustomKeyEventHandler((e) => {
    if (e.type !== 'keydown' || !e.ctrlKey || !e.shiftKey) return true;

    if (e.code === 'KeyC') {
      const selection = term.getSelection();
      // Nothing selected: fall through, so the chord still reaches the program.
      if (!selection) return true;
      // Returning false stops XTERM handling the key, not the BROWSER: without
      // this the webview also runs its native copy/paste for the same chord, so
      // the text was written twice (and pasted twice).
      e.preventDefault();
      copyText(selection).catch((err) => useStore.getState().setLastError(String(err)));
      return false;
    }
    if (e.code === 'KeyV') {
      e.preventDefault();
      invoke<string>('read_clipboard')
        // paste() goes through onData, so bracketed paste is honoured.
        .then((text) => { if (text) term.paste(text); })
        .catch((err) => useStore.getState().setLastError(String(err)));
      return false;
    }
    return true;
  });

  // Copy-on-select, debounced so a drag copies once when it settles rather than on
  // every intermediate change.
  let settle: number | undefined;
  let reported = false;
  term.onSelectionChange(() => {
    window.clearTimeout(settle);
    settle = window.setTimeout(() => {
      const selection = term.getSelection();
      if (!selection.trim()) return;
      copyText(selection).catch((err) => {
        // Once per session, not per drag: swallowing this entirely is what made a
        // broken clipboard look like a broken selection.
        if (reported) return;
        reported = true;
        useStore.getState().setLastError(`Clipboard: ${String(err)}`);
      });
    }, SELECTION_COPY_MS);
  });
}

/**
 * Draw on the GPU where the machine allows it.
 *
 * xterm's default renderer builds DOM for what it paints, which is the slowest part
 * of a terminal an agent is streaming into — it redraws its whole screen as it
 * thinks. The WebGL renderer draws from one texture atlas instead.
 *
 * Must be loaded AFTER `open`, and it can fail for reasons that are the machine's
 * rather than ours: no WebGL2, a driver that refuses, a context lost later on. Every
 * one of those falls back to the DOM renderer, which is what this app shipped with,
 * so the terminal is never worse off for having tried.
 */
function attachGpuRenderer(term: Terminal) {
  try {
    const gpu = new WebglAddon();
    gpu.onContextLoss(() => {
      // Nothing to retry against: the addon disposes itself and xterm reverts.
      console.warn('terminal: WebGL context lost, falling back to the DOM renderer');
      gpu.dispose();
    });
    term.loadAddon(gpu);
  } catch (e) {
    console.warn('terminal: no WebGL renderer, using the DOM one', e);
  }
}

/** Below this, a measurement is layout noise rather than a terminal. */
const MIN_COLS = 20;
const MIN_ROWS = 4;

/** The size each PTY was last told, so a fit that changes nothing costs no IPC. */
const syncedSize = new Map<string, { cols: number; rows: number }>();

/**
 * Resize a terminal and its shell together — or resize neither.
 *
 * `fit()` on its own is a bug, not a shortcut: it calls `term.resize`, which
 * REWRAPS the buffer and recomputes where the cursor sits. A shell that hears no
 * SIGWINCH keeps the old geometry, so its next redraw addresses a row that has
 * moved, and the line accumulates the characters it meant to overwrite. A prompt
 * long enough to wrap makes it happen on the very first keystroke.
 *
 * So the size is measured, checked, and applied to both sides here, and `fit()` is
 * called nowhere else.
 */
export function fitAndSync(sessionId: string) {
  const host = hosts.get(sessionId);
  const container = host?.el.parentElement;
  // Hidden panes and inactive sessions stay mounted at `display: none`, which
  // measures 0×0; fitting to that would rewrap every line for nothing.
  if (!host || !container || container.clientWidth < 2 || container.clientHeight < 2) return;

  let dims: { cols?: number; rows?: number } | undefined;
  try {
    dims = host.fit.proposeDimensions();
  } catch {
    return; // detached — it refits when it is next shown
  }
  const { cols, rows } = dims ?? {};
  if (!cols || !rows || cols < MIN_COLS || rows < MIN_ROWS) return;

  const last = syncedSize.get(sessionId);
  if (last?.cols === cols && last.rows === rows) return;
  syncedSize.set(sessionId, { cols, rows });

  host.term.resize(cols, rows);
  invoke('resize_pty', { sessionId, rows, cols }).catch((e) => {
    // Leaving the shell on a stale size is the corruption above, so this must not
    // be swallowed: drop the record so the next fit tries again.
    syncedSize.delete(sessionId);
    console.warn('resize_pty failed', e);
  });
}

/**
 * Change a metric-affecting option and make xterm redraw from scratch.
 *
 * A new size or family changes the cell box, and xterm caches glyph widths: after
 * only a resize the rows keep the OLD metrics, which paints characters at the
 * wrong offsets — a glyph can appear twice, most visibly at the start of a line.
 * Dropping the glyph cache and repainting every row is what discards that.
 */
function reflow(sessionId: string, host: TermHost, opts: { fontSize?: number; fontFamily?: string }) {
  if (opts.fontSize !== undefined) host.term.options.fontSize = opts.fontSize;
  if (opts.fontFamily !== undefined) host.term.options.fontFamily = opts.fontFamily;
  // New metrics mean a new column count, so the shell has to hear about it.
  syncedSize.delete(sessionId);
  fitAndSync(sessionId);
  host.term.clearTextureAtlas();
  host.term.refresh(0, host.term.rows - 1);
}

export function ensureHost(sessionId: string): TermHost {
  const existing = hosts.get(sessionId);
  if (existing) return existing;

  const term = new Terminal({
    fontFamily: termFontFamily(),
    fontSize: termFontSize(),
    lineHeight: 1.2,
    theme: xtermThemeFromCss(),
    cursorBlink: true,
    scrollback: 5000,
  });
  const fit = new FitAddon();
  term.loadAddon(fit);

  const el = document.createElement('div');
  el.className = 'pty-pane';
  term.open(el);
  attachGpuRenderer(term);
  attachClipboard(term);

  term.onData((data) => {
    const dataB64 = bytesToB64(new TextEncoder().encode(data));
    invoke('write_pty', { sessionId, dataB64 }).catch(console.error);
  });

  // Registered for the SESSION lifetime (not a component's): output keeps
  // flowing into the terminal's buffer even while no tab displays it.
  registerPtyHandler(sessionId, (bytes: Uint8Array) => {
    term.write(bytes);
  });

  const host: TermHost = { term, fit, el };
  hosts.set(sessionId, host);
  return host;
}

/** Tear a host down — on kill, endSession, or natural pty_exit. */
export function disposeHost(sessionId: string) {
  const host = hosts.get(sessionId);
  if (!host) return;
  hosts.delete(sessionId);
  syncedSize.delete(sessionId);
  unregisterPtyHandler(sessionId);
  host.el.remove();
  try { host.term.dispose(); } catch { /* already disposed */ }
}

export function focusHost(sessionId: string) {
  const host = hosts.get(sessionId);
  requestAnimationFrame(() => { try { host?.term.focus(); } catch { /* ignore */ } });
}

// Live re-skin on theme / font-size changes (no React needed).
let lastTheme: string | undefined;
let lastFont: number | undefined;
let lastFamily: string | undefined;
useStore.subscribe((state) => {
  const theme = state.config?.ui.theme;
  const font = state.config?.ui.font_size;
  const family = state.config?.ui.font_family;
  if (theme !== lastTheme) {
    lastTheme = theme;
    // data-theme lands on <html> in the same update; read after paint.
    requestAnimationFrame(() => {
      const skin = xtermThemeFromCss();
      for (const h of hosts.values()) h.term.options.theme = skin;
    });
  }
  if (font !== lastFont) {
    lastFont = font;
    const size = (font ?? DEFAULT_FONT_SIZE) + 1;
    for (const [id, h] of hosts) reflow(id, h, { fontSize: size });
  }
  if (family !== lastFamily) {
    lastFamily = family;
    const stack = termFontFamily();
    for (const [id, h] of hosts) reflow(id, h, { fontFamily: stack });
  }
});
