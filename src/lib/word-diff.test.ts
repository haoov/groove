import { describe, expect, it } from 'vitest';
import { wordDiff } from './word-diff';

// The ranges are character offsets used directly as CodeMirror decoration bounds,
// so an off-by-one paints the wrong characters or throws on an out-of-range mark.

/** Apply the ranges the way the editor does, marking them with « ». */
const marked = (s: string, ranges: [number, number][]) => {
  let out = '', at = 0;
  for (const [from, to] of ranges) {
    out += s.slice(at, from) + '«' + s.slice(from, to) + '»';
    at = to;
  }
  return out + s.slice(at);
};

describe('wordDiff', () => {
  it('marks only the word that changed', () => {
    const { delRanges, addRanges } = wordDiff('let a = 1;', 'let b = 1;');
    expect(marked('let a = 1;', delRanges)).toBe('let «a» = 1;');
    expect(marked('let b = 1;', addRanges)).toBe('let «b» = 1;');
  });

  it('marks nothing for identical lines', () => {
    const { delRanges, addRanges } = wordDiff('same', 'same');
    expect(delRanges).toEqual([]);
    expect(addRanges).toEqual([]);
  });

  it('marks the whole line when nothing is shared', () => {
    const { delRanges, addRanges } = wordDiff('aaa', 'bbb');
    expect(marked('aaa', delRanges)).toBe('«aaa»');
    expect(marked('bbb', addRanges)).toBe('«bbb»');
  });

  it('marks an addition with no deletion', () => {
    const { delRanges, addRanges } = wordDiff('foo', 'foo bar');
    expect(delRanges).toEqual([]);
    expect(marked('foo bar', addRanges)).toBe('foo« bar»');
  });

  it('handles an empty side', () => {
    expect(wordDiff('', 'new').delRanges).toEqual([]);
    expect(marked('new', wordDiff('', 'new').addRanges)).toBe('«new»');
    expect(marked('old', wordDiff('old', '').delRanges)).toBe('«old»');
  });

  it('produces ranges in order, non-overlapping, inside the string', () => {
    const a = 'const x = compute(a, b) + offset;';
    const b = 'const y = compute(a, c) - offset;';
    for (const [s, ranges] of [
      [a, wordDiff(a, b).delRanges],
      [b, wordDiff(a, b).addRanges],
    ] as const) {
      let prevEnd = 0;
      for (const [from, to] of ranges) {
        expect(from).toBeGreaterThanOrEqual(prevEnd);
        expect(to).toBeGreaterThan(from);
        expect(to).toBeLessThanOrEqual(s.length);
        prevEnd = to;
      }
    }
  });

  it('marks several separate changes on one line', () => {
    const a = 'foo(1) + bar(2)';
    const b = 'foo(9) + bar(8)';
    expect(marked(a, wordDiff(a, b).delRanges)).toBe('foo(«1») + bar(«2»)');
  });

  it('keeps leading indentation out of the marks when it is unchanged', () => {
    const a = '    return old;';
    const b = '    return new;';
    expect(marked(a, wordDiff(a, b).delRanges)).toBe('    return «old»;');
  });
});
