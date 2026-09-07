import type { Hunk, DiffLine } from '../shared/ipc/ipc';

/**
 * The unshown stretches of a diff, and how to fill one in.
 * `DiffLine.num` is the new-side number; a `del` line carries the number of the line it follows.
 */

/** A stretch of the new-side file that the diff does not show. */
export interface Gap {
  /** Index of the hunk this gap sits above; `hunks.length` for the trailing gap. */
  beforeHunk: number;
  /** 1-indexed, inclusive. */
  startLine: number;
  endLine: number;
}

/** How many lines one click reveals. */
export const GAP_STEP = 20;

const firstNum = (h: Hunk): number =>
  Math.min(...h.lines.filter((l) => l.type !== 'del').map((l) => l.num));
const lastNum = (h: Hunk): number =>
  Math.max(...h.lines.map((l) => l.num));

/** Every gap in the diff: above the first hunk, between each pair, and after the last when `total` is known.
 *  Hunks with no new-side line are skipped; `beforeHunk` indexes the original array. */
export function gapsFor(hunks: Hunk[], total?: number): Gap[] {
  const idx = hunks
    .map((h, i) => (h.lines.some((l) => l.type !== 'del') ? i : -1))
    .filter((i) => i >= 0);
  if (idx.length === 0) return [];
  const gaps: Gap[] = [];

  const leading = firstNum(hunks[idx[0]]);
  if (leading > 1) gaps.push({ beforeHunk: idx[0], startLine: 1, endLine: leading - 1 });

  for (let k = 0; k < idx.length - 1; k++) {
    const from = lastNum(hunks[idx[k]]) + 1;
    const to = firstNum(hunks[idx[k + 1]]) - 1;
    if (to >= from) gaps.push({ beforeHunk: idx[k + 1], startLine: from, endLine: to });
  }

  if (total !== undefined) {
    const from = lastNum(hunks[idx[idx.length - 1]]) + 1;
    if (total >= from) gaps.push({ beforeHunk: hunks.length, startLine: from, endLine: total });
  }
  return gaps;
}

/** The slice of a gap one click should fetch: `GAP_STEP` lines, or all of it. */
export function stepRange(gap: Gap, whole: boolean): { start: number; end: number } {
  if (whole || gap.endLine - gap.startLine + 1 <= GAP_STEP) {
    return { start: gap.startLine, end: gap.endLine };
  }
  // The gap at the top of the file grows upward from the hunk below it.
  return gap.startLine === 1
    ? { start: gap.endLine - GAP_STEP + 1, end: gap.endLine }
    : { start: gap.startLine, end: gap.startLine + GAP_STEP - 1 };
}

/** Splices fetched context into the hunk list. A fill that closes a gap merges the two hunks. */
export function mergeExpansion(
  hunks: Hunk[],
  gap: Gap,
  startLine: number,
  lines: string[],
): Hunk[] {
  if (lines.length === 0) return hunks;

  const filled: DiffLine[] = lines.map((content, i) => ({
    num: startLine + i,
    content,
    type: 'ctx',
  }));

  const out = hunks.map((h) => ({ ...h, lines: [...h.lines] }));
  const above = gap.beforeHunk - 1;
  const below = gap.beforeHunk;

  // Prefer the hunk above; the leading gap has none.
  if (above >= 0 && out[above]) {
    out[above].lines.push(...filled);
  } else if (out[below]) {
    out[below].lines.unshift(...filled);
  } else {
    return hunks;
  }

  // A closed gap folds the two hunks together.
  const closed = startLine === gap.startLine && startLine + lines.length - 1 === gap.endLine;
  if (closed && above >= 0 && out[below]) {
    out[above].lines.push(...out[below].lines);
    out.splice(below, 1);
  }
  return out;
}
