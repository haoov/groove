import { describe, expect, it } from 'vitest';
import { offeredSkills, offers, skillCommand, trimForPty } from './skills';
import type { AgentSkill, SessionKind } from '../ipc/ipc';

const skill = (id: string, kinds: SessionKind[]): AgentSkill => ({
  id,
  plugin: id.split(':')[0],
  name: id.split(':')[1],
  description: 'D',
  hint: 'H',
  label: 'L',
  kinds,
  editable: false,
});

// The space is what closes Claude's slash menu before the Enter lands, and the
// trim is the only thing that could eat it. Verified against a real PTY once;
// this is what keeps it verified.
describe('the text sent for a skill', () => {
  it('ends in a space when the skill takes no argument', () => {
    expect(skillCommand('groove:start-task')).toBe('/groove:start-task ');
    expect(trimForPty(skillCommand('groove:start-task'))).toMatch(/ $/);
  });

  it('puts an argument after that space', () => {
    expect(skillCommand('groove:create-task', 'notion')).toBe('/groove:create-task notion');
  });

  it('strips newlines and tabs but never the space', () => {
    expect(trimForPty('hello\n\n')).toBe('hello');
    expect(trimForPty('hello\t')).toBe('hello');
    expect(trimForPty('/groove:x ')).toBe('/groove:x ');
  });
});

describe('which skills a session offers', () => {
  const all = [
    skill('groove:start-task', ['task']),
    skill('groove:co-review', ['review']),
    skill('groove:new-skill', []),
  ];

  it('offers a skill only to the kinds it declares', () => {
    expect(offeredSkills(all, 'task').map((s) => s.id)).toEqual([
      'groove:start-task',
      'groove:new-skill',
    ]);
    expect(offeredSkills(all, 'review').map((s) => s.id)).toEqual([
      'groove:co-review',
      'groove:new-skill',
    ]);
  });

  /// A surface that names its own skill has to ask the same question the menu
  /// does — the CI chip offered `fix-ci` on a review session by checking only
  /// that the skill existed.
  it('answers for one skill the way the menu would', () => {
    expect(offers(all, 'task', 'groove:start-task')).toBe(true);
    expect(offers(all, 'review', 'groove:start-task')).toBe(false);
    expect(offers(all, 'task', 'groove:nothing')).toBe(false);
  });
});
