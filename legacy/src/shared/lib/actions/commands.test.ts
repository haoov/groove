import { describe, it, expect, vi } from 'vitest';

// The real module pulls xterm in, which needs a DOM.
vi.mock('./panes', () => ({ toggleTerminal: () => {} }));

import { COMMANDS, defaultKeymap, shortcutLabel, type CommandId } from '../pure/keybindings';
import { COMMAND_REGISTRY, commandRows } from './commands';

type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

const ALL = { session: true, workspace: true };

describe('COMMAND_REGISTRY', () => {
  // A row for an unknown id, or an id with no row, fails to compile here.
  it('covers exactly the CommandId union', () => {
    const covered: Same<keyof typeof COMMAND_REGISTRY, CommandId> = true;
    expect(covered).toBe(true);
  });

  it('holds exactly one entry per bound command', () => {
    expect(Object.keys(COMMAND_REGISTRY).sort()).toEqual(COMMANDS.map((c) => c.id).sort());
  });

  it('gives every entry an icon and a handler', () => {
    for (const [id, spec] of Object.entries(COMMAND_REGISTRY)) {
      expect(spec.icon, id).toBeTruthy();
      expect(typeof spec.run, id).toBe('function');
    }
  });
});

describe('commandRows', () => {
  it('offers every command but the palette itself', () => {
    const ids = commandRows(defaultKeymap(), ALL).map((r) => r.id);
    expect(ids).not.toContain('palette.commands');
    expect(ids).toContain('view.tasks');
    expect(ids.length).toBe(COMMANDS.length - 1);
  });

  it('keeps the keymap order and grouping', () => {
    const rows = commandRows(defaultKeymap(), ALL);
    const bound = COMMANDS.filter((c) => !COMMAND_REGISTRY[c.id].hidden);
    expect(rows.map((r) => `${r.group}/${r.label}`)).toEqual(bound.map((c) => `${c.group}/${c.label}`));
  });

  it('labels the shortcut from the keymap, not a literal', () => {
    const keymap = defaultKeymap();
    keymap['view.tasks'] = [{ key: 'F9' }];
    const row = commandRows(keymap, ALL).find((r) => r.id === 'view.tasks');
    expect(row?.shortcut).toBe(shortcutLabel(keymap, 'view.tasks'));
    expect(row?.shortcut).toBe('F9');
  });

  it('drops the session-scoped rows with no session', () => {
    const ids = commandRows(defaultKeymap(), { session: false, workspace: false }).map((r) => r.id);
    expect(ids).toContain('view.tasks');
    expect(ids).not.toContain('repo.add');
    expect(ids).not.toContain('pane.close');
  });

  it('drops the workspace-scoped rows outside a workspace', () => {
    const ids = commandRows(defaultKeymap(), { session: true, workspace: false }).map((r) => r.id);
    expect(ids).toContain('repo.add');
    expect(ids).not.toContain('pane.close');
  });
});
