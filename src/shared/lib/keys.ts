// ── Keyboard chord model ──────────────────────────────────────────────────────
// A chord is one key plus modifiers, matched against KeyboardEvent.key. `ctrl` matches Ctrl or Cmd.
// On macOS, Option chords resolve through `keyCode`.
// Do not use `.code`: it names a physical position and breaks non-QWERTY layouts.

import { isMac } from './platform';

// localStorage.setItem('wb.keyDebug', '1') logs what each Option chord resolved to.
let keyDebug = false;
try { keyDebug = localStorage.getItem('wb.keyDebug') === '1'; } catch { /* tests */ }

/** Letters and digits only; punctuation chords use `macDefaults` in keybindings.ts. */
function letterOrDigitFromKeyCode(keyCode: number): string | null {
  if (keyCode >= 65 && keyCode <= 90) return String.fromCharCode(keyCode + 32);
  if (keyCode >= 48 && keyCode <= 57) return String.fromCharCode(keyCode);
  return null;
}

function eventKey(e: KeyboardEvent): string {
  if (isMac() && e.altKey) {
    const resolved = letterOrDigitFromKeyCode(e.keyCode);
    if (keyDebug) {
      console.debug(`[key] alt key=${JSON.stringify(e.key)} code=${e.code} keyCode=${e.keyCode} -> ${resolved ?? normalizeKey(e.key)}`);
    }
    if (resolved) return resolved;
  }
  return normalizeKey(e.key);
}

export interface Chord {
  key: string; // normalized KeyboardEvent.key: lowercase, e.g. 'e', ';', 'tab', 'arrowup'
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
}

/** Normalizes a KeyboardEvent.key for storage and matching. */
export function normalizeKey(raw: string): string {
  if (raw === ' ' || raw === 'Spacebar') return 'space';
  return raw.toLowerCase();
}

const KEY_LABEL: Record<string, string> = {
  space: 'Space', tab: 'Tab', enter: 'Enter', escape: 'Esc',
  backspace: '⌫', delete: 'Del',
  arrowup: '↑', arrowdown: '↓', arrowleft: '←', arrowright: '→',
};

function keyLabel(k: string): string {
  if (KEY_LABEL[k]) return KEY_LABEL[k];
  if (k.length === 1) return k.toUpperCase();
  return k.charAt(0).toUpperCase() + k.slice(1);
}

/** Human-readable label: "Alt+Shift+E", or "⌥⇧E" on macOS. */
export function chordLabel(c: Chord): string {
  if (isMac()) {
    return `${c.alt ? '⌥' : ''}${c.shift ? '⇧' : ''}${c.ctrl ? '⌘' : ''}${keyLabel(c.key)}`;
  }
  const parts: string[] = [];
  if (c.ctrl) parts.push('Ctrl');
  if (c.alt) parts.push('Alt');
  if (c.shift) parts.push('Shift');
  parts.push(keyLabel(c.key));
  return parts.join('+');
}

export function chordFromEvent(e: KeyboardEvent): Chord {
  return {
    key: eventKey(e),
    ctrl: e.ctrlKey || e.metaKey,
    alt: e.altKey,
    shift: e.shiftKey,
  };
}

/** True while only a modifier key is held. */
export function isModifierOnly(e: KeyboardEvent): boolean {
  return e.key === 'Shift' || e.key === 'Control' || e.key === 'Alt' || e.key === 'Meta' || e.key === 'AltGraph';
}

/** True when a keystroke produces a character: a dead key composing, or AltGr held.
 *  No shortcut may take it. */
export function isTypingCharacter(e: KeyboardEvent): boolean {
  // macOS Option+E/I/N/U/` arrive as `Dead`; a resolvable Option chord is a command, not typing.
  if (isMac() && e.altKey && letterOrDigitFromKeyCode(e.keyCode)) return false;

  return (
    e.isComposing ||
    e.key === 'Dead' ||
    e.key === 'Unidentified' ||
    e.getModifierState('AltGraph')
  );
}

export function chordMatches(e: KeyboardEvent, c: Chord): boolean {
  return (
    eventKey(e) === c.key &&
    (e.ctrlKey || e.metaKey) === !!c.ctrl &&
    e.altKey === !!c.alt &&
    e.shiftKey === !!c.shift
  );
}
