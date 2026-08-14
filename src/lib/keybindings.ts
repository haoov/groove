import type { Chord } from './keys';

// ── Command registry + editable keymap ────────────────────────────────────────
// Single source of truth for every global shortcut. Bindings match the typed
// character (KeyboardEvent.key) so they follow the keyboard layout. The user's
// overrides live in localStorage and are merged over the defaults at load.

export type CommandId =
  | 'palette.commands'
  | 'files.quickOpen'
  | 'files.search'
  | 'settings.open'
  | 'panel.files'
  | 'panel.git'
  | 'panel.annotations'
  | 'git.cycleSubTab'
  | 'view.tasks'
  | 'view.notifications'
  | 'session.next'
  | 'session.prev'
  | 'workspace.toggleTerminal'
  | 'agent.console'
  | 'pane.splitRight'
  | 'pane.splitDown'
  | 'pane.close'
  | 'pane.next'
  | 'pane.maximize'
  | 'tab.next'
  | 'tab.prev'
  | 'tab.close'
  | 'session.switcher'
  | 'repo.switch'
  | 'repo.add'
  | 'git.commitFocus'
  | 'editor.focus'
  | 'editor.toggleVim'
  | 'editor.toggleBlame';

export interface CommandSpec {
  id: CommandId;
  label: string;
  group: string;
  defaults: Chord[];
}

const C = (key: string, mods: Partial<Chord> = {}): Chord => ({ key, ...mods });

export const COMMANDS: CommandSpec[] = [
  // General
  { id: 'palette.commands', label: 'Command palette', group: 'General', defaults: [C(':', { alt: true, shift: true })] },
  { id: 'files.quickOpen', label: 'Find file (search bar)', group: 'General', defaults: [C('f', { alt: true })] },
  { id: 'files.search', label: 'Search in files…', group: 'General', defaults: [C('f', { alt: true, shift: true })] },
  { id: 'settings.open', label: 'Open settings…', group: 'General', defaults: [C(',', { ctrl: true })] },

  // Panels
  { id: 'panel.files', label: 'Files tree', group: 'Panels', defaults: [C('e', { alt: true })] },
  { id: 'panel.git', label: 'Source control', group: 'Panels', defaults: [C('g', { alt: true })] },
  { id: 'panel.annotations', label: 'Annotations', group: 'Panels', defaults: [C('a', { ctrl: true, shift: true })] },
  { id: 'git.cycleSubTab', label: 'Cycle git sub-mode', group: 'Panels', defaults: [C('tab', { ctrl: true })] },
  { id: 'git.commitFocus', label: 'Write a commit message', group: 'Panels', defaults: [C('c', { alt: true, shift: true })] },

  // Navigation
  { id: 'view.tasks', label: 'Home', group: 'Navigation', defaults: [C('t', { alt: true })] },
  { id: 'view.notifications', label: 'Notifications', group: 'Navigation', defaults: [C('n', { ctrl: true })] },
  { id: 'session.next', label: 'Next session tab', group: 'Navigation', defaults: [C('n', { alt: true, shift: true })] },
  { id: 'session.switcher', label: 'Sessions dock (open / focus)', group: 'Navigation', defaults: [C('s', { alt: true })] },
  { id: 'session.prev', label: 'Previous session tab', group: 'Navigation', defaults: [C('p', { alt: true, shift: true })] },

  // Workspace
  { id: 'workspace.toggleTerminal', label: 'Terminal dock (open / focus / close)', group: 'Workspace', defaults: [C("'", { alt: true })] },
  { id: 'agent.console', label: 'Agent console (open / focus)', group: 'Workspace', defaults: [C('a', { alt: true })] },
  { id: 'pane.splitRight', label: 'Split pane right', group: 'Workspace', defaults: [C('|', { alt: true, shift: true })] },
  { id: 'pane.splitDown', label: 'Split pane down', group: 'Workspace', defaults: [C('-', { alt: true })] },
  { id: 'pane.close', label: 'Close pane', group: 'Workspace', defaults: [C('w', { alt: true, shift: true })] },
  { id: 'pane.next', label: 'Focus next pane', group: 'Workspace', defaults: [C('o', { alt: true })] },
  { id: 'pane.maximize', label: 'Maximize / restore pane', group: 'Workspace', defaults: [C('m', { alt: true })] },
  { id: 'tab.next', label: 'Next file tab', group: 'Workspace', defaults: [C('n', { alt: true })] },
  { id: 'tab.prev', label: 'Previous file tab', group: 'Workspace', defaults: [C('p', { alt: true })] },
  { id: 'tab.close', label: 'Close file tab', group: 'Workspace', defaults: [C('w', { alt: true })] },
  { id: 'repo.switch', label: 'Switch repo…', group: 'Workspace', defaults: [C('r', { alt: true })] },
  { id: 'repo.add', label: 'Add a repo to this session…', group: 'Workspace', defaults: [C('r', { alt: true, shift: true })] },

  // Editor
  { id: 'editor.focus', label: 'Focus editor', group: 'Editor', defaults: [C('c', { alt: true })] },
  { id: 'editor.toggleVim', label: 'Toggle Vim mode', group: 'Editor', defaults: [C('v', { alt: true, shift: true })] },
  { id: 'editor.toggleBlame', label: 'Toggle blame gutter', group: 'Editor', defaults: [C('b', { alt: true })] },
];

export type Keymap = Record<CommandId, Chord[]>;

export function defaultKeymap(): Keymap {
  const m = {} as Keymap;
  for (const c of COMMANDS) m[c.id] = c.defaults.map((d) => ({ ...d }));
  return m;
}

const LS_KEY = 'workbench.keymap.v4';
/** Older maps are read once and migrated. v1 → v2 resolved chords shared by two
 *  commands (they used to resolve silently by declaration order); v2 → v3 released
 *  chords whose owning command changed (see MOVED_CHORDS). */
const LS_KEYS_OLD = ['workbench.keymap.v3', 'workbench.keymap.v2', 'workbench.keymap.v1'];

/**
 * Commands whose default chord moved to a different command, so a stored binding
 * has to be dropped rather than kept: keeping it would either hold the chord
 * hostage or lose the command its new default.
 *
 * `repo.add` held Alt+R until `repo.switch` took it; add-repo is now Alt+Shift+R.
 * `pane.close` lost Alt+W to `tab.close` and was left with nothing, so it takes
 * its new Alt+Shift+W default rather than staying unbound.
 */
const MOVED_CHORDS: CommandId[] = ['repo.add', 'pane.close'];

/** A chord identifies one command. Same shape as `chordMatches` compares. */
const chordKey = (c: Chord) =>
  `${c.key}|${!!c.ctrl}|${!!c.alt}|${!!c.shift}`;

/**
 * When a chord is claimed by more than one command, the winner used to be
 * whichever was declared first in COMMANDS — invisible in Settings and wrong as
 * soon as a new command shipped with a default someone had already bound.
 *
 * `tab.close` over `pane.close` is the case that actually happened: Alt+W was
 * bound to "close the pane" when that was the only close there was, then
 * "close the file" arrived wanting the same chord. Closing a file is what the
 * chord is expected to do; the pane keeps its button and can be rebound.
 */
const CHORD_WINNERS: CommandId[] = ['tab.close'];

/** Strip a chord from every command but its rightful owner. */
function resolveConflicts(map: Keymap): Keymap {
  const claims = new Map<string, CommandId[]>();
  for (const id of Object.keys(map) as CommandId[]) {
    for (const chord of map[id] ?? []) {
      const key = chordKey(chord);
      claims.set(key, [...(claims.get(key) ?? []), id]);
    }
  }

  const order = COMMANDS.map((c) => c.id);
  const out: Keymap = { ...map };
  for (const [key, owners] of claims) {
    if (owners.length < 2) continue;
    // An explicit winner, else the runtime's historical answer (declared first)
    // so resolving a conflict never silently moves an unrelated shortcut.
    const winner =
      owners.find((id) => CHORD_WINNERS.includes(id)) ??
      owners.slice().sort((a, b) => order.indexOf(a) - order.indexOf(b))[0];
    for (const loser of owners) {
      if (loser === winner) continue;
      out[loser] = (out[loser] ?? []).filter((c) => chordKey(c) !== key);
    }
  }
  return out;
}

export function loadKeymap(): Keymap {
  const base = defaultKeymap();
  try {
    // v2 if present, else migrate v1 in place: a heavily customised map must not
    // be thrown away just because conflict handling changed.
    const current = localStorage.getItem(LS_KEY);
    const raw = current ?? LS_KEYS_OLD.map((k) => localStorage.getItem(k)).find(Boolean);
    if (!raw) return base;
    const saved = JSON.parse(raw) as Partial<Record<CommandId, Chord[]>>;
    for (const id of Object.keys(saved) as CommandId[]) {
      // On an upgrade, a moved chord keeps the NEW default instead of the stored one.
      if (!current && MOVED_CHORDS.includes(id)) continue;
      if (base[id] && Array.isArray(saved[id])) base[id] = saved[id]!;
    }
    const resolved = resolveConflicts(base);
    if (!current) saveKeymap(resolved);
    return resolved;
  } catch {
    /* corrupt — fall back to defaults */
  }
  return base;
}

/** Assign `chords` to `id`, taking them off any command that already had them.
 *  Two commands on one chord is not a state the UI should be able to create. */
export function assignBinding(map: Keymap, id: CommandId, chords: Chord[]): Keymap {
  const taken = new Set(chords.map(chordKey));
  const out: Keymap = { ...map, [id]: chords };
  for (const other of Object.keys(out) as CommandId[]) {
    if (other === id) continue;
    out[other] = (out[other] ?? []).filter((c) => !taken.has(chordKey(c)));
  }
  return out;
}

export function saveKeymap(m: Keymap): void {
  try { localStorage.setItem(LS_KEY, JSON.stringify(m)); } catch { /* ignore */ }
}

export function clearKeymap(): void {
  try { localStorage.removeItem(LS_KEY); } catch { /* ignore */ }
}
