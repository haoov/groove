import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { COMMANDS, assignBinding, defaultKeymap, loadKeymap, saveKeymap, type CommandId, type Keymap } from './keybindings';
import { chordLabel, chordMatches, isTypingCharacter, normalizeKey, type Chord } from './keys';
import { setPlatform } from './platform';

// The keymap is the one piece of state that survives upgrades: a stored map from an
// older version is merged over new defaults. Every bug here has been the same
// shape — a chord ending up on two commands, or a command silently losing its key.

const store = new Map<string, string>();

beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => { store.set(k, v); },
    removeItem: (k: string) => { store.delete(k); },
  });
});

const key = (c: Chord) => `${c.key}|${!!c.ctrl}|${!!c.alt}|${!!c.shift}`;

/** Every chord in the map, with the commands claiming it. */
function claims(map: Keymap): Map<string, string[]> {
  const m = new Map<string, string[]>();
  for (const [id, chords] of Object.entries(map)) {
    for (const c of chords ?? []) m.set(key(c), [...(m.get(key(c)) ?? []), id]);
  }
  return m;
}

describe('defaultKeymap', () => {
  it('gives every declared command its defaults', () => {
    const m = defaultKeymap();
    for (const c of COMMANDS) expect(m[c.id], c.id).toEqual(c.defaults);
  });

  it('ships with no chord claimed by two commands', () => {
    const doubled = [...claims(defaultKeymap())].filter(([, ids]) => ids.length > 1);
    expect(doubled, `shared chords: ${JSON.stringify(doubled)}`).toEqual([]);
  });

  it('leaves every command bound except tab.close', () => {
    const m = defaultKeymap();
    const unbound = COMMANDS.filter((c) => (m[c.id] ?? []).length === 0).map((c) => c.id);
    // tab.close is intentionally unbound: Alt+W now cycles worktrees, and a tab
    // closes with middle-click or its × button.
    expect(unbound).toEqual(['tab.close']);
  });
});

describe('loadKeymap', () => {
  it('returns the defaults with nothing stored', () => {
    expect(loadKeymap()).toEqual(defaultKeymap());
  });

  it('keeps a stored override', () => {
    saveKeymap({ ...defaultKeymap(), 'editor.focus': [{ key: 'q', alt: true }] });
    expect(loadKeymap()['editor.focus']).toEqual([{ key: 'q', alt: true }]);
  });

  it('gives a command added after the stored map its new default', () => {
    // A v4 map written before editor.toggleBlame existed.
    const old = { ...defaultKeymap() } as Record<string, Chord[]>;
    delete old['editor.toggleBlame'];
    saveKeymap(old as Keymap);
    expect(loadKeymap()['editor.toggleBlame']).toEqual([{ key: 'b', alt: true }]);
  });

  it('ignores a command id that no longer exists', () => {
    saveKeymap({ ...defaultKeymap(), 'gone.command': [{ key: 'z' }] } as unknown as Keymap);
    expect(loadKeymap()).not.toHaveProperty('gone.command');
  });

  it('falls back to the defaults on a corrupt map', () => {
    localStorage.setItem('workbench.keymap.v4', '{not json');
    expect(loadKeymap()).toEqual(defaultKeymap());
  });

  it('never returns a chord shared by two commands', () => {
    // A stored map that claims a chord another command now owns by default.
    saveKeymap({ ...defaultKeymap(), 'editor.focus': [{ key: 'w', alt: true }] });
    const doubled = [...claims(loadKeymap())].filter(([, ids]) => ids.length > 1);
    expect(doubled, `shared chords: ${JSON.stringify(doubled)}`).toEqual([]);
  });

  it('gives Alt+W to the worktree switcher, not tab or pane close', () => {
    const m = loadKeymap();
    const hasAltW = (id: CommandId) => (m[id] ?? []).some((c) => c.key === 'w' && c.alt && !c.shift);
    expect(hasAltW('worktree.switch')).toBe(true);
    expect(hasAltW('tab.close')).toBe(false);
    expect(hasAltW('pane.close')).toBe(false);
  });

  it('migrates a v3 map and drops the chords that changed owner', () => {
    // v3 had repo.add on Alt+R; repo.switch owns it now.
    const v3 = { ...defaultKeymap(), 'repo.add': [{ key: 'r', alt: true }] };
    localStorage.setItem('workbench.keymap.v3', JSON.stringify(v3));
    const m = loadKeymap();
    expect(m['repo.switch'].some((c) => c.key === 'r' && c.alt && !c.shift)).toBe(true);
    expect(m['repo.add'].some((c) => c.key === 'r' && c.alt && !c.shift)).toBe(false);
    expect(m['repo.add'].length).toBeGreaterThan(0);
  });

  it('writes the migrated map back, so the migration runs once', () => {
    localStorage.setItem('workbench.keymap.v4', JSON.stringify(defaultKeymap()));
    loadKeymap();
    expect(store.has('workbench.keymap.v6')).toBe(true);
  });
});

describe('assignBinding', () => {
  it('takes the chord off whichever command had it', () => {
    const m = assignBinding(defaultKeymap(), 'editor.focus', [{ key: 'w', alt: true }]);
    expect(m['editor.focus']).toEqual([{ key: 'w', alt: true }]);
    expect(m['tab.close'].some((c) => c.key === 'w' && c.alt && !c.shift)).toBe(false);
  });

  it('can leave a command with no binding at all', () => {
    let m = defaultKeymap();
    m = assignBinding(m, 'editor.focus', []);
    expect(m['editor.focus']).toEqual([]);
  });

  it('never creates a shared chord', () => {
    let m = defaultKeymap();
    for (const c of COMMANDS) m = assignBinding(m, c.id, [{ key: 'k', ctrl: true }]);
    const doubled = [...claims(m)].filter(([, ids]) => ids.length > 1);
    expect(doubled).toEqual([]);
  });

  it('distinguishes chords that differ only by a modifier', () => {
    const m = assignBinding(defaultKeymap(), 'editor.focus', [{ key: 'w', alt: true, shift: true }]);
    // Alt+Shift+W belonged to pane.close; Alt+W (worktree.switch) is untouched.
    expect(m['worktree.switch'].some((c) => c.key === 'w' && c.alt && !c.shift)).toBe(true);
    expect((m['pane.close'] ?? []).some((c) => c.key === 'w' && c.alt && c.shift)).toBe(false);
  });
});

describe('chord model', () => {
  const ev = (o: Partial<KeyboardEvent>) => o as KeyboardEvent;

  it('normalizes the space bar and letter case', () => {
    expect(normalizeKey(' ')).toBe('space');
    expect(normalizeKey('Spacebar')).toBe('space');
    expect(normalizeKey('E')).toBe('e');
    expect(normalizeKey('ArrowUp')).toBe('arrowup');
  });

  it('matches on the typed character, so layouts do not matter', () => {
    const c: Chord = { key: ';', alt: true };
    expect(chordMatches(ev({ key: ';', altKey: true, ctrlKey: false, metaKey: false, shiftKey: false }), c)).toBe(true);
    expect(chordMatches(ev({ key: ';', altKey: false, ctrlKey: false, metaKey: false, shiftKey: false }), c)).toBe(false);
  });

  it('treats Cmd as Ctrl', () => {
    const c: Chord = { key: 'p', ctrl: true };
    expect(chordMatches(ev({ key: 'p', metaKey: true, ctrlKey: false, altKey: false, shiftKey: false }), c)).toBe(true);
  });

  it('requires an unset modifier to be absent', () => {
    const c: Chord = { key: 'n', alt: true };
    expect(chordMatches(ev({ key: 'n', altKey: true, shiftKey: true, ctrlKey: false, metaKey: false }), c)).toBe(false);
  });

  it('labels a chord the way Settings shows it', () => {
    expect(chordLabel({ key: 'e', alt: true, shift: true })).toBe('Alt+Shift+E');
    expect(chordLabel({ key: ';', ctrl: true })).toBe('Ctrl+;');
    expect(chordLabel({ key: 'tab', alt: true })).toBe('Alt+Tab');
    expect(chordLabel({ key: 'arrowup' })).toBe('↑');
  });
});

// The keymap listens in the capture phase, ahead of whatever has focus. On an
// international layout that is where characters go missing: the reported case was
// `´` then space, which is how you type a bare apostrophe there.
describe('isTypingCharacter', () => {
  const ev = (o: Partial<KeyboardEvent> & { altGraph?: boolean }) =>
    ({ getModifierState: (m: string) => m === 'AltGraph' && !!o.altGraph, ...o }) as KeyboardEvent;

  it('lets a dead key and everything it composes through', () => {
    expect(isTypingCharacter(ev({ key: 'Dead' }))).toBe(true);
    // `´` then space, and `´` then e: the second keydown is mid-composition.
    expect(isTypingCharacter(ev({ key: ' ', isComposing: true }))).toBe(true);
    expect(isTypingCharacter(ev({ key: 'e', isComposing: true }))).toBe(true);
  });

  it('lets an AltGr character through, however the browser reports it', () => {
    // Reported as Alt on some engines, as Ctrl+Alt on others.
    expect(isTypingCharacter(ev({ key: '@', altKey: true, altGraph: true }))).toBe(true);
    expect(isTypingCharacter(ev({ key: '@', altKey: true, ctrlKey: true, altGraph: true }))).toBe(true);
  });

  it('leaves real chords alone', () => {
    expect(isTypingCharacter(ev({ key: 'n', altKey: true }))).toBe(false);
    expect(isTypingCharacter(ev({ key: 'k', ctrlKey: true }))).toBe(false);
    expect(isTypingCharacter(ev({ key: ' ' }))).toBe(false);
    expect(isTypingCharacter(ev({ key: "'" }))).toBe(false);
  });
});

describe('macOS chords', () => {
  const ev = (o: Partial<KeyboardEvent>) => o as KeyboardEvent;
  /** Option+<letter>: a composed character, with a keyCode naming the real key. */
  const optionPress = (composed: string, keyCode: number, extra: Partial<KeyboardEvent> = {}) =>
    ev({ key: composed, keyCode, altKey: true, ctrlKey: false, metaKey: false, shiftKey: false,
         isComposing: false, getModifierState: () => false, ...extra });

  beforeEach(() => { setPlatform('macos'); });
  afterEach(() => { setPlatform('linux'); });

  it('resolves the letter behind a composed Option character', () => {
    expect(chordMatches(optionPress('ß', 83), { key: 's', alt: true })).toBe(true);
    expect(chordMatches(optionPress('ß', 83), { key: 'ß', alt: true })).toBe(false);
  });

  it('follows the layout, not the physical key', () => {
    // On AZERTY the key QWERTY calls Q is labelled A; keyCode reports 65.
    expect(chordMatches(optionPress('æ', 65), { key: 'a', alt: true })).toBe(true);
    expect(chordMatches(optionPress('æ', 65), { key: 'q', alt: true })).toBe(false);
  });

  it('lets a dead-key Option chord through as a command', () => {
    // Option+E is a dead acute, and panel.files binds it.
    const deadE = optionPress('Dead', 69);
    expect(isTypingCharacter(deadE)).toBe(false);
    expect(chordMatches(deadE, { key: 'e', alt: true })).toBe(true);
  });

  it('still protects real composition that is not an Option chord', () => {
    const dead = ev({ key: 'Dead', keyCode: 192, altKey: false, isComposing: false,
                      getModifierState: () => false });
    expect(isTypingCharacter(dead)).toBe(true);
  });

  it('leaves punctuation to the platform defaults rather than guessing', () => {
    expect(chordMatches(optionPress('æ', 222), { key: "'", alt: true })).toBe(false);
  });

  it('gives the diverged commands a reachable default', () => {
    const mac = defaultKeymap();
    for (const c of COMMANDS.filter((c) => c.macDefaults)) {
      for (const chord of mac[c.id]) {
        expect(chord.key, `${c.id} must not bind punctuation under Option`)
          .toMatch(/^[a-z0-9]$/);
      }
    }
  });

  it('keeps every macOS default free of conflicts', () => {
    const doubled = [...claims(defaultKeymap())].filter(([, ids]) => ids.length > 1);
    expect(doubled).toEqual([]);
  });

  it('labels chords with Apple glyphs', () => {
    expect(chordLabel({ key: 'e', alt: true, shift: true })).toBe('⌥⇧E');
    expect(chordLabel({ key: ',', ctrl: true })).toBe('⌘,');
  });
});

describe('Linux defaults are untouched by the port', () => {
  it('keeps the punctuation chords the diverged commands always had', () => {
    const linux = defaultKeymap(); // platform defaults to linux in these tests
    expect(linux['palette.commands']).toEqual([{ key: ':', alt: true, shift: true }]);
    expect(linux['workspace.toggleTerminal']).toEqual([{ key: "'", alt: true }]);
    expect(linux['pane.splitRight']).toEqual([{ key: '|', alt: true, shift: true }]);
    expect(linux['pane.splitDown']).toEqual([{ key: '-', alt: true }]);
  });
});
