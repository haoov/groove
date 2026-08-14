import { describe, expect, it } from 'vitest';
import { buildDocument } from './diffDoc';
import type { DiffLine, Hunk } from '../../types/ipc';

// The diff editor renders every hunk into ONE document, so a CodeMirror line number
// is not a file line number. `lineMap` is the only translation between them, and
// every anchor in the diff — comments, MR threads, blame, the expansion bands —
// reads through it. An off-by-one here puts a comment on the wrong line.

const ctx = (n: number, c = `ctx ${n}`): DiffLine => ({ num: n, content: c, type: 'ctx' });
const add = (n: number, c = `add ${n}`): DiffLine => ({ num: n, content: c, type: 'add' });
const del = (n: number, c = `del ${n}`): DiffLine => ({ num: n, content: c, type: 'del' });

const hunks: Hunk[] = [
  { header: '@@ -8,4 +8,5 @@', lines: [ctx(8), del(8), add(9), ctx(10)] },
  { header: '@@ -40,2 +41,2 @@', lines: [ctx(41), ctx(42)] },
];

describe('buildDocument', () => {
  it('lays every hunk into one document, in order', () => {
    const { doc } = buildDocument(hunks);
    expect(doc.split('\n')).toEqual(['ctx 8', 'del 8', 'add 9', 'ctx 10', 'ctx 41', 'ctx 42']);
  });

  it('maps each document line to its file line and type', () => {
    const { lineMap } = buildDocument(hunks);
    expect(lineMap).toEqual([
      { fileLineNum: 8, type: 'ctx' },
      { fileLineNum: 8, type: 'del' },
      { fileLineNum: 9, type: 'add' },
      { fileLineNum: 10, type: 'ctx' },
      { fileLineNum: 41, type: 'ctx' },
      { fileLineNum: 42, type: 'ctx' },
    ]);
  });

  it('gives one lineMap entry per document line', () => {
    const { doc, lineMap } = buildDocument(hunks);
    expect(lineMap).toHaveLength(doc.split('\n').length);
  });

  // The gap bands and hunk separators are placed at these lines.
  it('reports where each hunk starts, 1-indexed', () => {
    const { hunkFirstCMLines } = buildDocument(hunks);
    expect(hunkFirstCMLines).toEqual([1, 5]);
  });

  it('keeps content verbatim, including blank and whitespace-only lines', () => {
    const { doc, lineMap } = buildDocument([
      { header: '@@', lines: [ctx(1, ''), ctx(2, '   '), add(3, '\tindented')] },
    ]);
    expect(doc.split('\n')).toEqual(['', '   ', '\tindented']);
    expect(lineMap[2]).toEqual({ fileLineNum: 3, type: 'add' });
  });

  it('handles a single hunk and an empty list', () => {
    expect(buildDocument([{ header: '@@', lines: [ctx(1)] }]).hunkFirstCMLines).toEqual([1]);
    const empty = buildDocument([]);
    expect(empty.doc).toBe('');
    expect(empty.lineMap).toEqual([]);
    expect(empty.hunkFirstCMLines).toEqual([]);
  });

  it('survives a hunk with no lines without shifting the next one', () => {
    const { hunkFirstCMLines, lineMap } = buildDocument([
      { header: '@@ a', lines: [ctx(1)] },
      { header: '@@ empty', lines: [] },
      { header: '@@ b', lines: [ctx(5)] },
    ]);
    expect(hunkFirstCMLines).toEqual([1, 2, 2]);
    expect(lineMap[1]).toEqual({ fileLineNum: 5, type: 'ctx' });
  });

  // Expansion writes merged hunks back through the same builder, so the mapping has
  // to hold for a hunk whose context was filled in after the fact.
  it('maps an expanded hunk the same way', () => {
    const expanded: Hunk[] = [
      { header: '@@', lines: [ctx(8), del(8), add(9), ctx(10), ctx(11), ctx(12)] },
    ];
    const { lineMap } = buildDocument(expanded);
    expect(lineMap.map((l) => l.fileLineNum)).toEqual([8, 8, 9, 10, 11, 12]);
    expect(lineMap.filter((l) => l.type === 'del')).toHaveLength(1);
  });
});
