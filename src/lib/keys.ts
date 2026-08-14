// ── Keyboard chord model ──────────────────────────────────────────────────────
// A chord is one key plus modifiers, matched against KeyboardEvent.key (the
// CHARACTER produced, normalized) rather than .code (a physical position). This
// keeps shortcuts correct across layouts (AZERTY/QWERTY): the binding follows the
// letter/symbol you actually type. `ctrl` matches Ctrl (Win/Linux) OR Cmd (mac).

export interface Chord {
  key: string; // normalized KeyboardEvent.key: lowercase, e.g. 'e', ';', 'tab', 'arrowup'
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
}

/** Normalize a KeyboardEvent.key for storage/matching (case- and space-stable). */
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

/** Human-readable label, e.g. "Alt+Shift+E" or "Ctrl+;". */
export function chordLabel(c: Chord): string {
  const parts: string[] = [];
  if (c.ctrl) parts.push('Ctrl');
  if (c.alt) parts.push('Alt');
  if (c.shift) parts.push('Shift');
  parts.push(keyLabel(c.key));
  return parts.join('+');
}

export function chordFromEvent(e: KeyboardEvent): Chord {
  return {
    key: normalizeKey(e.key),
    ctrl: e.ctrlKey || e.metaKey,
    alt: e.altKey,
    shift: e.shiftKey,
  };
}

/** True while only a modifier key is held (no "real" key yet). */
export function isModifierOnly(e: KeyboardEvent): boolean {
  return e.key === 'Shift' || e.key === 'Control' || e.key === 'Alt' || e.key === 'Meta' || e.key === 'AltGraph';
}

export function chordMatches(e: KeyboardEvent, c: Chord): boolean {
  return (
    normalizeKey(e.key) === c.key &&
    (e.ctrlKey || e.metaKey) === !!c.ctrl &&
    e.altKey === !!c.alt &&
    e.shiftKey === !!c.shift
  );
}
