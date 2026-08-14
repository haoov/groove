import { describe, expect, it } from 'vitest';
import { STATUS_RANK, statusKey } from './taskStatus';

// Notion boards name statuses freely, so every label the team actually uses has to
// land on a known key — anything unrecognized silently becomes "ready" and sorts
// into the queue as if it were actionable.

describe('statusKey', () => {
  it.each([
    ['In progress', 'in_progress'],
    ['in progress', 'in_progress'],
    ['IN PROGRESS', 'in_progress'],
    ['Progressing', 'in_progress'],
    ['Blocked', 'blocked'],
    ['Blocked by upstream', 'blocked'],
    ['In review', 'in_review'],
    ['Ready for review', 'in_review'],
    ['Done', 'done'],
    ['Complete', 'done'],
    ['Completed', 'done'],
    ['Ready for sprint', 'ready'],
    ['Backlog', 'ready'],
    ['', 'ready'],
  ])('maps %j to %j', (label, key) => {
    expect(statusKey(label)).toBe(key);
  });

  it('is stable under extra whitespace', () => {
    expect(statusKey('In   Progress')).toBe('in_progress');
  });

  // "Fixed with required action" contains neither "done" nor "review": it stays
  // actionable on purpose, since it still needs someone to do the action.
  it('keeps a status that only sounds finished in the queue', () => {
    expect(statusKey('Fixed with required action')).toBe('ready');
  });

  it('prefers progress over review when a label says both', () => {
    expect(statusKey('In progress — needs review')).toBe('in_progress');
  });
});

describe('STATUS_RANK', () => {
  it('sorts what you are doing above what you could do', () => {
    const order = ['in_progress', 'ready', 'blocked', 'in_review'];
    const ranks = order.map((k) => STATUS_RANK[k]);
    expect(ranks).toEqual([...ranks].sort((a, b) => a - b));
  });

  it('leaves done unranked, so it falls to the end of the queue', () => {
    expect(STATUS_RANK.done).toBeUndefined();
  });

  it('ranks every non-done key statusKey can return', () => {
    const keys = ['In progress', 'Blocked', 'In review', 'Backlog'].map(statusKey);
    for (const k of keys) expect(STATUS_RANK[k], k).toBeTypeOf('number');
  });
});
