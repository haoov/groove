import { describe, expect, it } from 'vitest';
import { annotationsForStartLine, deriveAnnotationSets, threadsForStartLine } from './annotationSets';
import type { Annotation, MrThread } from '../../shared/ipc/ipc';

// Both editors draw their gutter indicators from these sets, so a mistake here
// puts a comment marker on the wrong line in the diff AND in the editor.

const ann = (start: number, end = start, file = 'a.ts'): Annotation => ({
  id: `a${start}-${end}`, session_id: 't', repo_id: 'r', file_path: file,
  start_line: start, end_line: end,
  content: 'note', author: 'me', status: 'open', created_at: 0,
});

const thread = (
  file: string,
  pos: { new_line?: number; line_range?: { end?: { new_line?: number } } },
  resolved?: boolean,
): MrThread => ({
  id: 'd1',
  notes: [{ body: 'x', resolved, position: { new_path: file, ...pos } }],
});

describe('deriveAnnotationSets', () => {
  it('marks the first line of a range as its start, and every line as annotated', () => {
    const s = deriveAnnotationSets([ann(10, 13)], [], 'a.ts');
    expect([...s.annStartNums]).toEqual([10]);
    expect([...s.annotatedLineNums]).toEqual([10, 11, 12, 13]);
  });

  it('handles a single-line annotation', () => {
    const s = deriveAnnotationSets([ann(7)], [], 'a.ts');
    expect([...s.annStartNums]).toEqual([7]);
    expect([...s.annotatedLineNums]).toEqual([7]);
  });

  it('merges overlapping annotations without duplicating lines', () => {
    const s = deriveAnnotationSets([ann(1, 3), ann(2, 4)], [], 'a.ts');
    expect([...s.annotatedLineNums].sort((a, b) => a - b)).toEqual([1, 2, 3, 4]);
    expect([...s.annStartNums].sort((a, b) => a - b)).toEqual([1, 2]);
  });

  it('ignores threads positioned in another file', () => {
    const s = deriveAnnotationSets([], [thread('other.ts', { new_line: 5 })], 'a.ts');
    expect(s.threadNums.size).toBe(0);
  });

  it('takes a multi-line thread at the end of its range, where GitLab anchors it', () => {
    const s = deriveAnnotationSets([], [thread('a.ts', { line_range: { end: { new_line: 12 } } })], 'a.ts');
    expect([...s.threadNums]).toEqual([12]);
  });

  it('prefers new_line over the range end when both are present', () => {
    const t = thread('a.ts', { new_line: 5, line_range: { end: { new_line: 12 } } });
    const s = deriveAnnotationSets([], [t], 'a.ts');
    expect([...s.threadNums]).toEqual([5]);
  });

  it('drops a thread with no usable line', () => {
    const s = deriveAnnotationSets([], [thread('a.ts', {})], 'a.ts');
    expect(s.threadNums.size).toBe(0);
  });

  // resolved is optional in the API payload: a note that never says so counts as
  // unresolved, which is why the check is `!== true` rather than `=== false`.
  it('counts a thread as unresolved unless every note says resolved', () => {
    const unset = deriveAnnotationSets([], [thread('a.ts', { new_line: 5 })], 'a.ts');
    expect([...unset.unresolvedThreadNums]).toEqual([5]);

    const done = deriveAnnotationSets([], [thread('a.ts', { new_line: 5 }, true)], 'a.ts');
    expect(done.unresolvedThreadNums.size).toBe(0);
    expect([...done.threadNums]).toEqual([5]);
  });

  it('treats a thread as unresolved when only some notes are resolved', () => {
    const mixed: MrThread = {
      id: 'd',
      notes: [
        { body: 'a', resolved: true, position: { new_path: 'a.ts', new_line: 9 } },
        { body: 'b', resolved: false },
      ],
    };
    const s = deriveAnnotationSets([], [mixed], 'a.ts');
    expect([...s.unresolvedThreadNums]).toEqual([9]);
  });

  it('positions a thread by its FIRST note only', () => {
    const t: MrThread = {
      id: 'd',
      notes: [
        { body: 'a', position: { new_path: 'a.ts', new_line: 3 } },
        { body: 'reply', position: { new_path: 'a.ts', new_line: 99 } },
      ],
    };
    expect([...deriveAnnotationSets([], [t], 'a.ts').threadNums]).toEqual([3]);
  });

  it('survives a thread with no notes at all', () => {
    expect(() => deriveAnnotationSets([], [{ id: 'd' }], 'a.ts')).not.toThrow();
  });
});

describe('annotationsForStartLine', () => {
  it('takes only the annotations anchored on that line', () => {
    const found = annotationsForStartLine([ann(10, 13), ann(11), ann(10)], 10);
    expect(found.map((a) => a.id)).toEqual(['a10-13', 'a10-10']);
  });
});

describe('threadsForStartLine', () => {
  it('matches either the line or the end of a range, in the right file', () => {
    const threads = [
      thread('a.ts', { new_line: 5 }),
      thread('a.ts', { line_range: { end: { new_line: 9 } } }),
      thread('b.ts', { new_line: 5 }),
    ];
    expect(threadsForStartLine(threads, 'a.ts', 5)).toHaveLength(1);
    expect(threadsForStartLine(threads, 'a.ts', 9)).toHaveLength(1);
    expect(threadsForStartLine(threads, 'b.ts', 9)).toHaveLength(0);
  });
});
