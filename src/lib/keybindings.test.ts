import { beforeEach, describe, expect, it, vi } from 'vitest';
import { COMMANDS, assignBinding, defaultKeymap, loadKeymap, saveKeymap, type Keymap } from './keybindings';
import { chordLabel, chordMatches, normalizeKey, type Chord } from './keys';

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

  it('leaves no command unbound', () => {
    const m = defaultKeymap();
    const unbound = COMMANDS.filter((c) => (m[c.id] ?? []).length === 0).map((c) => c.id);
    expect(unbound).toEqual([]);
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

  it('gives Alt+W to closing a file, not closing a pane', () => {
    // The real regression: Alt+W was the pane-close chord before file tabs existed.
    const m = loadKeymap();
    const altW = m['tab.close'].some((c) => c.key === 'w' && c.alt && !c.shift);
    const paneAltW = (m['pane.close'] ?? []).some((c) => c.key === 'w' && c.alt && !c.shift);
    expect(altW).toBe(true);
    expect(paneAltW).toBe(false);
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
    localStorage.setItem('workbench.keymap.v3', JSON.stringify(defaultKeymap()));
    loadKeymap();
    expect(store.has('workbench.keymap.v4')).toBe(true);
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
    // Alt+Shift+W belonged to pane.close; Alt+W (tab.close) is untouched.
    expect(m['tab.close'].some((c) => c.key === 'w' && c.alt && !c.shift)).toBe(true);
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
