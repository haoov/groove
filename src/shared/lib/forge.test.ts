import { describe, expect, it } from 'vitest';
import { forgeName, mrRef, mrSigil } from './forge';

describe('forge helpers', () => {
  it('maps each forge to its sigil and name', () => {
    expect(mrSigil('github')).toBe('#');
    expect(mrSigil('gitlab')).toBe('!');
    expect(mrRef('github', 42)).toBe('#42');
    expect(mrRef('gitlab', '7')).toBe('!7');
    expect(forgeName('github')).toBe('GitHub');
    expect(forgeName('gitlab')).toBe('GitLab');
  });
});
