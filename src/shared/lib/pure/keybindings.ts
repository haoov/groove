import { chordLabel, type Chord } from './keys';
import { isMac } from './platform';

// ── Command registry + editable keymap ────────────────────────────────────────
// Single source of truth for every global shortcut. Bindings match KeyboardEvent.key.

export type CommandId =
  | 'palette.commands'
  | 'files.quickOpen'
  | 'files.search'
  | 'settings.open'
  | 'panel.overview'
  | 'panel.files'
  | 'panel.git'
  | 'panel.annotations'
  | 'git.cycleSubTab'
  | 'view.tasks'
  | 'view.notifications'
  | 'agents.sidebar'
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
  | 'worktree.switch'
  | 'repo.add'
  | 'git.commitFocus'
  | 'editor.focus'
  | 'editor.toggleVim'
  | 'editor.toggleBlame'
  | 'font.increase'
  | 'font.decrease'
  | 'font.reset';

export interface CommandSpec {
  id: CommandId;
  label: string;
  group: string;
  defaults: Chord[];
  /** macOS override, for chords whose Linux key is punctuation Option composes. */
  macDefaults?: Chord[];
}

const C = (key: string, mods: Partial<Chord> = {}): Chord => ({ key, ...mods });

export const COMMANDS: CommandSpec[] = [
  // General
  { id: 'palette.commands', label: 'Command palette', group: 'General', defaults: [C(':', { alt: true, shift: true })], macDefaults: [C('k', { alt: true, shift: true })] },
  { id: 'files.quickOpen', label: 'Find file (search bar)', group: 'General', defaults: [C('f', { alt: true })] },
  { id: 'files.search', label: 'Search in files…', group: 'General', defaults: [C('f', { alt: true, shift: true })] },
  { id: 'settings.open', label: 'Open settings…', group: 'General', defaults: [C(',', { ctrl: true })] },

  // Panels
  { id: 'panel.overview', label: 'Overview', group: 'Panels', defaults: [C('o', { alt: true })] },
  { id: 'panel.files', label: 'Files tree', group: 'Panels', defaults: [C('e', { alt: true })] },
  { id: 'panel.git', label: 'Source control', group: 'Panels', defaults: [C('g', { alt: true })] },
  { id: 'panel.annotations', label: 'Notes', group: 'Panels', defaults: [C('a', { ctrl: true, shift: true })] },
  { id: 'git.cycleSubTab', label: 'Cycle git sub-mode', group: 'Panels', defaults: [C('tab', { ctrl: true })] },
  { id: 'git.commitFocus', label: 'Write a commit message', group: 'Panels', defaults: [C('c', { alt: true, shift: true })] },

  // Navigation
  { id: 'view.tasks', label: 'Home', group: 'Navigation', defaults: [C('t', { alt: true })] },
  { id: 'view.notifications', label: 'Notifications', group: 'Navigation', defaults: [C('n', { ctrl: true })] },
  { id: 'agents.sidebar', label: 'Running agents (show / hide)', group: 'Navigation', defaults: [C('a', { alt: true, shift: true })] },
  { id: 'session.next', label: 'Next session tab', group: 'Navigation', defaults: [C('n', { alt: true, shift: true })] },
  { id: 'session.switcher', label: 'Session switcher (open / cycle)', group: 'Navigation', defaults: [C('s', { alt: true })] },
  { id: 'session.prev', label: 'Previous session tab', group: 'Navigation', defaults: [C('p', { alt: true, shift: true })] },

  // Workspace
  { id: 'workspace.toggleTerminal', label: 'Terminal dock (open / focus / close)', group: 'Workspace', defaults: [C("'", { alt: true })], macDefaults: [C('j', { alt: true })] },
  { id: 'agent.console', label: 'Agent console (open / focus)', group: 'Workspace', defaults: [C('a', { alt: true })] },
  { id: 'pane.splitRight', label: 'Split pane right', group: 'Workspace', defaults: [C('|', { alt: true, shift: true })], macDefaults: [C('d', { alt: true })] },
  { id: 'pane.splitDown', label: 'Split pane down', group: 'Workspace', defaults: [C('-', { alt: true })], macDefaults: [C('d', { alt: true, shift: true })] },
  { id: 'pane.close', label: 'Close pane', group: 'Workspace', defaults: [C('w', { alt: true, shift: true })] },
  { id: 'pane.next', label: 'Focus next pane', group: 'Workspace', defaults: [C('i', { alt: true })] },
  { id: 'pane.maximize', label: 'Maximize / restore pane', group: 'Workspace', defaults: [C('m', { alt: true })] },
  { id: 'tab.next', label: 'Next file tab', group: 'Workspace', defaults: [C('n', { alt: true })] },
  { id: 'tab.prev', label: 'Previous file tab', group: 'Workspace', defaults: [C('p', { alt: true })] },
  { id: 'tab.close', label: 'Close file tab', group: 'Workspace', defaults: [] },
  { id: 'repo.switch', label: 'Repo switcher (open / cycle)', group: 'Workspace', defaults: [C('r', { alt: true })] },
  { id: 'worktree.switch', label: 'Worktree switcher (open / cycle)', group: 'Workspace', defaults: [C('w', { alt: true })] },
  { id: 'repo.add', label: 'Add a repo to this session…', group: 'Workspace', defaults: [C('r', { alt: true, shift: true })] },

  // Editor
  { id: 'editor.focus', label: 'Focus editor', group: 'Editor', defaults: [C('c', { alt: true })] },
  { id: 'editor.toggleVim', label: 'Toggle Vim mode', group: 'Editor', defaults: [C('v', { alt: true, shift: true })] },
  { id: 'editor.toggleBlame', label: 'Toggle blame gutter', group: 'Editor', defaults: [C('b', { alt: true })] },
  { id: 'font.increase', label: 'Increase font size', group: 'Editor', defaults: [C('=', { ctrl: true }), C('+', { ctrl: true, shift: true })] },
  { id: 'font.decrease', label: 'Decrease font size', group: 'Editor', defaults: [C('-', { ctrl: true })] },
  { id: 'font.reset', label: 'Reset font size', group: 'Editor', defaults: [C('0', { ctrl: true })] },
];

export type Keymap = Record<CommandId, Chord[]>;

export function defaultKeymap(): Keymap {
  const mac = isMac();
  const m = {} as Keymap;
  for (const c of COMMANDS) {
    const src = mac && c.macDefaults ? c.macDefaults : c.defaults;
    m[c.id] = src.map((d) => ({ ...d }));
  }
  return m;
}

/** Commands whose default differs on macOS. */
const macDivergedIds = (): CommandId[] =>
  COMMANDS.filter((c) => c.macDefaults).map((c) => c.id);

const LS_KEY = 'workbench.keymap.v7';
/** Older maps are read once and migrated. */
const LS_KEYS_OLD = ['workbench.keymap.v6', 'workbench.keymap.v5', 'workbench.keymap.v4', 'workbench.keymap.v3', 'workbench.keymap.v2', 'workbench.keymap.v1'];

/** Commands whose default chord moved; an upgrade drops their stored binding. */
const MOVED_CHORDS: CommandId[] = ['repo.add', 'pane.close', 'tab.close', 'pane.next'];

/** Chord identity. Keep it aligned with what `chordMatches` compares. */
const chordKey = (c: Chord) =>
  `${c.key}|${!!c.ctrl}|${!!c.alt}|${!!c.shift}`;

/** Explicit winners when two commands claim one chord. */
const CHORD_WINNERS: CommandId[] = ['tab.close'];

/** Strips a chord from every command but its owner. */
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
    // An explicit winner, else the first declared.
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
    // The current key if present, else migrate an older map in place.
    const current = localStorage.getItem(LS_KEY);
    const raw = current ?? LS_KEYS_OLD.map((k) => localStorage.getItem(k)).find(Boolean);
    if (!raw) return base;
    const saved = JSON.parse(raw) as Partial<Record<CommandId, Chord[]>>;
    for (const id of Object.keys(saved) as CommandId[]) {
      // On an upgrade, a moved chord keeps the new default.
      if (!current && MOVED_CHORDS.includes(id)) continue;
      // A pre-macOS map holds punctuation chords Option cannot produce.
      if (!current && isMac() && macDivergedIds().includes(id)) continue;
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

/** Assigns `chords` to `id` and takes them off every other command. */
export function assignBinding(map: Keymap, id: CommandId, chords: Chord[]): Keymap {
  const taken = new Set(chords.map(chordKey));
  const out: Keymap = { ...map, [id]: chords };
  for (const other of Object.keys(out) as CommandId[]) {
    if (other === id) continue;
    out[other] = (out[other] ?? []).filter((c) => !taken.has(chordKey(c)));
  }
  return out;
}

/** A command's spec, by id. */
export const commandSpec = (id: CommandId): CommandSpec | undefined =>
  COMMANDS.find((c) => c.id === id);

/** This platform's defaults for one command. */
export function defaultChordsFor(id: CommandId): Chord[] {
  const spec = commandSpec(id);
  if (!spec) return [];
  const src = isMac() && spec.macDefaults ? spec.macDefaults : spec.defaults;
  return src.map((d) => ({ ...d }));
}

/** True when a command still holds this platform's defaults. */
export function isDefaultBinding(map: Keymap, id: CommandId): boolean {
  const a = map[id] ?? [];
  const b = defaultChordsFor(id);
  return a.length === b.length && a.every((c, i) => chordKey(c) === chordKey(b[i]));
}

/** Puts one command back on its platform default; the chord comes off whatever holds it. */
export function resetBinding(map: Keymap, id: CommandId): Keymap {
  return assignBinding(map, id, defaultChordsFor(id));
}

/** The other command already holding `chord`, or null when it is free. */
export function chordOwner(map: Keymap, chord: Chord, except: CommandId): CommandId | null {
  const k = chordKey(chord);
  for (const id of Object.keys(map) as CommandId[]) {
    if (id === except) continue;
    if ((map[id] ?? []).some((c) => chordKey(c) === k)) return id;
  }
  return null;
}

/** Everything a command can be found by: its group, its label, its chords. */
const searchText = (spec: CommandSpec, chords: Chord[]): string =>
  [spec.group, spec.label, ...chords.map(chordLabel)].join(' ').toLowerCase();

/** Commands matching every whitespace-separated term in `query`. Empty = all. */
export function searchCommands(query: string, map: Keymap): CommandSpec[] {
  const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return COMMANDS;
  return COMMANDS.filter((spec) => {
    const text = searchText(spec, map[spec.id] ?? []);
    return terms.every((t) => text.includes(t));
  });
}

export function saveKeymap(m: Keymap): void {
  try { localStorage.setItem(LS_KEY, JSON.stringify(m)); } catch { /* ignore */ }
}

export function clearKeymap(): void {
  try { localStorage.removeItem(LS_KEY); } catch { /* ignore */ }
}

/** First chord bound to a command, labelled for this platform. Use it for every shortcut hint. */
export function shortcutLabel(keymap: Keymap, id: CommandId): string | undefined {
  const c = keymap[id]?.[0];
  return c ? chordLabel(c) : undefined;
}
