import { describe, expect, it } from 'vitest';
import { GAP_STEP, gapsFor, mergeExpansion, stepRange } from './diffGaps';
import type { DiffLine, Hunk } from '../ipc/ipc';

const ctx = (n: number): DiffLine => ({ num: n, content: `line ${n}`, type: 'ctx' });
const add = (n: number): DiffLine => ({ num: n, content: `added ${n}`, type: 'add' });
const del = (n: number): DiffLine => ({ num: n, content: 'removed', type: 'del' });

/** Two hunks, 10-13 and 40-43, in a 60-line file. */
const twoHunks = (): Hunk[] => [
  { header: '@@', lines: [ctx(10), add(11), ctx(12), ctx(13)] },
  { header: '@@', lines: [ctx(40), del(40), add(41), ctx(42), ctx(43)] },
];

const keys = (hunks: Hunk[]) => hunks.flatMap((h) => h.lines.map((l) => `${l.type}:${l.num}`));

describe('gapsFor', () => {
  it('finds the gap above, between and after the hunks', () => {
    expect(gapsFor(twoHunks(), 60)).toEqual([
      { beforeHunk: 0, startLine: 1, endLine: 9 },
      { beforeHunk: 1, startLine: 14, endLine: 39 },
      { beforeHunk: 2, startLine: 44, endLine: 60 },
    ]);
  });

  it('omits the trailing gap when the file length is unknown', () => {
    expect(gapsFor(twoHunks()).map((g) => g.beforeHunk)).toEqual([0, 1]);
  });

  it('reports nothing when the hunk already covers the file', () => {
    expect(gapsFor([{ header: '@@', lines: [ctx(1), ctx(2)] }], 2)).toEqual([]);
  });

  it('reports a one-line gap', () => {
    const hunks = [{ header: '@@', lines: [ctx(5)] }, { header: '@@', lines: [ctx(7)] }];
    expect(gapsFor(hunks, 7)).toEqual([
      { beforeHunk: 0, startLine: 1, endLine: 4 },
      { beforeHunk: 1, startLine: 6, endLine: 6 },
    ]);
  });

  // A del line carries the new-side number of the line it was removed after, so
  // it must never extend the hunk's range and hide a line from the gap.
  it('ignores del lines when measuring a hunk', () => {
    const hunks = [
      { header: '@@', lines: [ctx(10), del(10), del(10)] },
      { header: '@@', lines: [ctx(20)] },
    ];
    expect(gapsFor(hunks, 20)[1]).toEqual({ beforeHunk: 1, startLine: 11, endLine: 19 });
  });

  // A whole-file deletion occupies no new-side range. Indices must still point at
  // the original array, or the widget lands above the wrong hunk.
  it('skips a hunk with no new-side line but keeps the original indices', () => {
    const hunks = [
      { header: '@@', lines: [del(0)] },
      { header: '@@', lines: [ctx(10), add(11)] },
      { header: '@@', lines: [ctx(40)] },
    ];
    expect(gapsFor(hunks, 50)).toEqual([
      { beforeHunk: 1, startLine: 1, endLine: 9 },
      { beforeHunk: 2, startLine: 12, endLine: 39 },
      { beforeHunk: 3, startLine: 41, endLine: 50 },
    ]);
  });

  it('returns nothing when no hunk has a new side', () => {
    expect(gapsFor([{ header: '@@', lines: [del(0)] }], 5)).toEqual([]);
  });
});

describe('stepRange', () => {
  const gaps = gapsFor(twoHunks(), 60);

  it('grows the top gap upward, so the lines nearest the change come first', () => {
    const big = { beforeHunk: 0, startLine: 1, endLine: 100 };
    expect(stepRange(big, false)).toEqual({ start: 81, end: 100 });
  });

  it('grows a middle gap downward', () => {
    expect(stepRange(gaps[1], false)).toEqual({ start: 14, end: 14 + GAP_STEP - 1 });
  });

  it('takes the whole gap on demand', () => {
    expect(stepRange(gaps[1], true)).toEqual({ start: 14, end: 39 });
  });

  it('takes a gap smaller than one step whole, without overshooting', () => {
    expect(stepRange(gaps[0], false)).toEqual({ start: 1, end: 9 });
  });
});

describe('mergeExpansion', () => {
  // The property annotations and MR threads depend on: they anchor on the new-side
  // line number, so an expansion must never renumber a line that already existed.
  it('keeps every pre-existing line at its own number', () => {
    const hunks = twoHunks();
    const gap = gapsFor(hunks, 60)[1];
    const merged = mergeExpansion(hunks, gap, 14, ['a', 'b', 'c']);
    for (const k of keys(hunks)) expect(keys(merged)).toContain(k);
  });

  it('numbers the fetched lines from the start of the fetch', () => {
    const hunks = twoHunks();
    const gap = gapsFor(hunks, 60)[1];
    const merged = mergeExpansion(hunks, gap, 14, ['a', 'b', 'c']);
    expect(merged[0].lines.slice(-3)).toEqual([
      { num: 14, content: 'a', type: 'ctx' },
      { num: 15, content: 'b', type: 'ctx' },
      { num: 16, content: 'c', type: 'ctx' },
    ]);
  });

  it('leaves the gap open when it only filled part of it', () => {
    const hunks = twoHunks();
    const gap = gapsFor(hunks, 60)[1];
    const merged = mergeExpansion(hunks, gap, 14, ['a', 'b', 'c']);
    expect(merged).toHaveLength(2);
    expect(gapsFor(merged, 60)[1]).toEqual({ beforeHunk: 1, startLine: 17, endLine: 39 });
  });

  it('merges the two hunks when the fill closes the gap', () => {
    const hunks = twoHunks();
    const gap = gapsFor(hunks, 60)[1];
    const filled = Array.from({ length: 26 }, (_, i) => `f${i}`);
    const merged = mergeExpansion(hunks, gap, 14, filled);
    expect(merged).toHaveLength(1);
    const nums = merged[0].lines.filter((l) => l.type !== 'del').map((l) => l.num);
    expect(nums[0]).toBe(10);
    expect(nums[nums.length - 1]).toBe(43);
    expect(new Set(nums).size).toBe(nums.length);
    expect(gapsFor(merged, 60)).toEqual([
      { beforeHunk: 0, startLine: 1, endLine: 9 },
      { beforeHunk: 1, startLine: 44, endLine: 60 },
    ]);
  });

  it('puts a top-of-file fill before the first hunk', () => {
    const hunks = twoHunks();
    const gap = gapsFor(hunks, 60)[0];
    const merged = mergeExpansion(hunks, gap, 1, Array.from({ length: 9 }, (_, i) => `h${i}`));
    expect(merged[0].lines[0].num).toBe(1);
    expect(gapsFor(merged, 60).some((g) => g.startLine === 1)).toBe(false);
  });

  it('appends a trailing fill to the last hunk', () => {
    const hunks = twoHunks();
    const gap = gapsFor(hunks, 60)[2];
    const merged = mergeExpansion(hunks, gap, 44, ['t1', 't2']);
    const last = merged[1].lines[merged[1].lines.length - 1];
    expect(last).toEqual({ num: 45, content: 't2', type: 'ctx' });
    const gaps = gapsFor(merged, 60);
    expect(gaps[gaps.length - 1]).toEqual({ beforeHunk: 2, startLine: 46, endLine: 60 });
  });

  it('is a no-op for an empty fetch', () => {
    const hunks = twoHunks();
    expect(mergeExpansion(hunks, gapsFor(hunks, 60)[1], 14, [])).toBe(hunks);
  });

  it('does not mutate the hunks it was given', () => {
    const hunks = twoHunks();
    const before = JSON.stringify(hunks);
    mergeExpansion(hunks, gapsFor(hunks, 60)[1], 14, ['a']);
    expect(JSON.stringify(hunks)).toBe(before);
  });
});
