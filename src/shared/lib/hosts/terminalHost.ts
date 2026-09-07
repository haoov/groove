import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { invoke } from '../../ipc/invoke';
import { useStore } from '../../store';
import { DEFAULT_FONT_SIZE } from '../../ipc/ipc';
import { registerPtyHandler, unregisterPtyHandler, bytesToB64 } from './ptyRegistry';
import { errorText } from '../pure/appError';
import '@xterm/xterm/css/xterm.css';

/** A terminal that lives for the PTY session, outside React.
 *  Do not re-`open()` a Terminal; components only re-parent `el`. */
export interface TermHost {
  term: Terminal;
  fit: FitAddon;
  el: HTMLDivElement;
  /** The agent's terminal: its own family and a smaller size than the shells. */
  agent: boolean;
}

const hosts = new Map<string, TermHost>();

/** Resolves the active theme's CSS custom properties into an xterm theme object. */
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

const baseFontSize = () => useStore.getState().config?.ui.font_size ?? DEFAULT_FONT_SIZE;
const termFontSize = (agent: boolean) => baseFontSize() + (agent ? -1 : 1);

/** The bundled stack, also the agent's fallback. */
const MONO_STACK = `'Lilex', 'IBM Plex Mono', ui-monospace, monospace`;
const agentFontFamily = () => {
  const configured = useStore.getState().config?.ui.agent_font_family?.trim();
  return configured ? `'${configured}', ${MONO_STACK}` : MONO_STACK;
};

/** The configured family with the `--font-mono` fallbacks. Keep the two in sync. */
const termFontFamily = () => {
  const configured = useStore.getState().config?.ui.font_family?.trim();
  const stack = MONO_STACK;
  return configured ? `'${configured}', ${stack}` : stack;
};

/** Clipboard through the backend. `navigator.clipboard` and `execCommand('copy')`
 *  both fail silently in WebKitGTK. */
async function copyText(text: string): Promise<void> {
  await invoke('copy_to_clipboard', { text });
}

/** Copy and paste inside a terminal: Ctrl+Shift+C / Ctrl+Shift+V, plus copy-on-select. */
function attachClipboard(term: Terminal) {
  term.attachCustomKeyEventHandler((e) => {
    if (e.type !== 'keydown' || !e.ctrlKey || !e.shiftKey) return true;

    if (e.code === 'KeyC') {
      const selection = term.getSelection();
      // Nothing selected: the chord reaches the program.
      if (!selection) return true;
      // Returning false stops xterm only; preventDefault stops the webview's native copy.
      e.preventDefault();
      copyText(selection).catch((err) => useStore.getState().setLastError(err));
      return false;
    }
    if (e.code === 'KeyV') {
      e.preventDefault();
      invoke<string>('read_clipboard')
        .then((text) => { if (text) term.paste(text); })
        .catch((err) => useStore.getState().setLastError(err));
      return false;
    }
    return true;
  });

  term.element?.addEventListener('paste', (e) => {
    // The webview also pastes on this event; cancel it.
    e.preventDefault();
    // With mouse tracking on, the program pastes the middle click itself.
    if (term.modes.mouseTrackingMode !== 'none') e.stopPropagation();
  }, true);

  // Copy-on-select, debounced.
  let settle: number | undefined;
  let reported = false;
  term.onSelectionChange(() => {
    window.clearTimeout(settle);
    settle = window.setTimeout(() => {
      const selection = term.getSelection();
      if (!selection.trim()) return;
      copyText(selection).catch((err) => {
        // Report once per session.
        if (reported) return;
        reported = true;
        useStore.getState().setLastError(`Clipboard: ${errorText(err)}`);
      });
    }, SELECTION_COPY_MS);
  });
}

/** Loads the WebGL renderer. Load it after `open`; any failure falls back to the DOM renderer. */
function attachGpuRenderer(term: Terminal) {
  try {
    const gpu = new WebglAddon();
    gpu.onContextLoss(() => {
      console.warn('terminal: WebGL context lost, falling back to the DOM renderer');
      gpu.dispose();
    });
    term.loadAddon(gpu);
  } catch (e) {
    console.warn('terminal: no WebGL renderer, using the DOM one', e);
  }
}

/** Below this, a measurement is layout noise. */
const MIN_COLS = 20;
const MIN_ROWS = 4;

/** The size each PTY was last told, so a fit that changes nothing costs no IPC. */
const syncedSize = new Map<string, { cols: number; rows: number }>();

/** Drops the size record; the next fit resends the size. Another window can have resized the PTY. */
export function forgetSyncedSize(sessionId: string) {
  syncedSize.delete(sessionId);
}

/** Resizes a terminal and its shell together, or neither.
 *  Never call `fit()` elsewhere: a rewrapped buffer with an un-notified shell corrupts the screen. */
export function fitAndSync(sessionId: string) {
  const host = hosts.get(sessionId);
  const container = host?.el.parentElement;
  // Hidden panes measure 0×0; do not fit to that.
  if (!host || !container || container.clientWidth < 2 || container.clientHeight < 2) return;

  let dims: { cols?: number; rows?: number } | undefined;
  try {
    dims = host.fit.proposeDimensions();
  } catch {
    return; // detached; refits when next shown
  }
  const { cols, rows } = dims ?? {};
  if (!cols || !rows || cols < MIN_COLS || rows < MIN_ROWS) return;

  const last = syncedSize.get(sessionId);
  if (last?.cols === cols && last.rows === rows) return;
  syncedSize.set(sessionId, { cols, rows });

  host.term.resize(cols, rows);
  invoke('resize_pty', { sessionId, rows, cols }).catch((e) => {
    // Drop the record; the next fit retries.
    syncedSize.delete(sessionId);
    console.warn('resize_pty failed', e);
  });
}

/** Applies a metric-affecting option, refits, and repaints every row.
 *  Keep `clearTextureAtlas`: xterm caches glyph widths and paints stale offsets after a resize. */
function reflow(sessionId: string, host: TermHost, opts: { fontSize?: number; fontFamily?: string }) {
  if (opts.fontSize !== undefined) host.term.options.fontSize = opts.fontSize;
  if (opts.fontFamily !== undefined) host.term.options.fontFamily = opts.fontFamily;
  // New metrics change the column count; force a resync.
  syncedSize.delete(sessionId);
  fitAndSync(sessionId);
  host.term.clearTextureAtlas();
  host.term.refresh(0, host.term.rows - 1);
}

export function ensureHost(sessionId: string, agent = false): TermHost {
  const existing = hosts.get(sessionId);
  if (existing) return existing;

  const term = new Terminal({
    fontFamily: agent ? agentFontFamily() : termFontFamily(),
    fontSize: termFontSize(agent),
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

  // Registered for the session lifetime, not a component's.
  registerPtyHandler(sessionId, (bytes: Uint8Array) => {
    term.write(bytes);
  });

  const host: TermHost = { term, fit, el, agent };
  hosts.set(sessionId, host);
  return host;
}

/** Tears a host down: kill, endSession, or pty_exit. */
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

// Live re-skin on theme / font changes.
let lastTheme: string | undefined;
let lastFont: number | undefined;
let lastFamily: string | undefined;
let lastAgentFamily: string | undefined;
useStore.subscribe((state) => {
  const theme = state.config?.ui.theme;
  const font = state.config?.ui.font_size;
  const family = state.config?.ui.font_family;
  const agentFamily = state.config?.ui.agent_font_family;
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
    for (const [id, h] of hosts) reflow(id, h, { fontSize: termFontSize(h.agent) });
  }
  if (family !== lastFamily) {
    lastFamily = family;
    const stack = termFontFamily();
    for (const [id, h] of hosts) if (!h.agent) reflow(id, h, { fontFamily: stack });
  }
  if (agentFamily !== lastAgentFamily) {
    lastAgentFamily = agentFamily;
    const stack = agentFontFamily();
    for (const [id, h] of hosts) if (h.agent) reflow(id, h, { fontFamily: stack });
  }
});
