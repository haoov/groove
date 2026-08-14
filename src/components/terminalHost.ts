import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { invoke } from '@tauri-apps/api/core';
import { useStore } from '../store';
import { DEFAULT_FONT_SIZE } from '../types/ipc';
import { registerPtyHandler, unregisterPtyHandler } from '../hooks/useIpc';
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
 * Change a metric-affecting option and make xterm redraw from scratch.
 *
 * A new size or family changes the cell box, and xterm caches glyph widths: after
 * only `fit()` the rows keep the OLD metrics, which paints characters at the wrong
 * offsets — a glyph can appear twice, most visibly at the start of a line. The
 * explicit `refresh` of every row is what discards that.
 */
function reflow(host: TermHost, opts: { fontSize?: number; fontFamily?: string }) {
  if (opts.fontSize !== undefined) host.term.options.fontSize = opts.fontSize;
  if (opts.fontFamily !== undefined) host.term.options.fontFamily = opts.fontFamily;
  try {
    host.fit.fit();
    host.term.refresh(0, host.term.rows - 1);
  } catch { /* detached — it refits when it is next shown */ }
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
  attachClipboard(term);

  term.onData((data) => {
    const bytes = Array.from(new TextEncoder().encode(data));
    invoke('write_pty', { sessionId, data: bytes }).catch(console.error);
  });

  // Registered for the SESSION lifetime (not a component's): output keeps
  // flowing into the terminal's buffer even while no tab displays it.
  registerPtyHandler(sessionId, (bytes: number[]) => {
    term.write(new Uint8Array(bytes));
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
    for (const h of hosts.values()) reflow(h, { fontSize: size });
  }
  if (family !== lastFamily) {
    lastFamily = family;
    const stack = termFontFamily();
    for (const h of hosts.values()) reflow(h, { fontFamily: stack });
  }
});
