import { Fragment, type ReactNode } from 'react';

/** Case-insensitive match ranges for highlighting.
 *  Prefers a contiguous substring; falls back to a fuzzy subsequence.
 *  Returns matched [start, end) ranges, [] for an empty query, or null when there's no match. */
export function matchRanges(query: string, text: string): [number, number][] | null {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const t = text.toLowerCase();

  const sub = t.indexOf(q);
  if (sub !== -1) return [[sub, sub + q.length]];

  // subsequence — group contiguous hits into runs
  const ranges: [number, number][] = [];
  let qi = 0;
  let runStart = -1;
  for (let i = 0; i < t.length && qi < q.length; i++) {
    if (t[i] === q[qi]) {
      if (runStart === -1) runStart = i;
      qi++;
      if (qi === q.length || t[i + 1] !== q[qi]) {
        ranges.push([runStart, i + 1]);
        runStart = -1;
      }
    }
  }
  return qi === q.length ? ranges : null;
}

/** Render text with matched ranges wrapped in <mark className="hl-match">. */
export function Highlighted({ text, ranges }: { text: string; ranges: [number, number][] | null }) {
  if (!ranges || ranges.length === 0) return <>{text}</>;
  const out: ReactNode[] = [];
  let pos = 0;
  ranges.forEach(([s, e], idx) => {
    if (s > pos) out.push(<Fragment key={`p${idx}`}>{text.slice(pos, s)}</Fragment>);
    out.push(<mark key={`m${idx}`} className="hl-match">{text.slice(s, e)}</mark>);
    pos = e;
  });
  if (pos < text.length) out.push(<Fragment key="tail">{text.slice(pos)}</Fragment>);
  return <>{out}</>;
}

/** A word start: the first character, or one after a separator. */
function isBoundary(text: string, i: number): boolean {
  return i === 0 || /[/\-_. ]/.test(text[i - 1]);
}

/** Contiguous runs the ranges cover, and how far the match spans. */
function shape(ranges: [number, number][]): { runs: number; span: number } {
  if (ranges.length === 0) return { runs: 0, span: 0 };
  return {
    runs: ranges.length,
    span: ranges[ranges.length - 1][1] - ranges[0][0],
  };
}

/**
 * A scattered subsequence is a match in name only: `mayo` hits half a repo list
 * through m…a…y…o. Accept one only when it arrives in few enough runs, which
 * scales with the query so long queries may still break across words.
 */
function tightEnough(query: string, runs: number): boolean {
  return runs <= Math.max(2, Math.ceil(query.length / 2));
}

export type Ranked<T> = { item: T; ranges: [number, number][] };

/**
 * Items that match `query`, best first. Contiguous hits outrank scattered ones,
 * and a hit at a word start outranks one mid-word.
 *
 * Prefer this to calling `matchRanges` in a filter: that keeps every subsequence
 * however scattered, and leaves the order to chance.
 */
export function rankMatches<T>(
  query: string,
  items: readonly T[],
  toText: (item: T) => string,
): Ranked<T>[] {
  const q = query.trim().toLowerCase();
  if (!q) return items.map((item) => ({ item, ranges: [] as [number, number][] }));

  const scored: { item: T; ranges: [number, number][]; score: number }[] = [];
  items.forEach((item) => {
    const text = toText(item);
    const ranges = matchRanges(q, text);
    if (!ranges || ranges.length === 0) return;

    const { runs, span } = shape(ranges);
    const start = ranges[0][0];
    const contiguous = runs === 1;
    if (!contiguous && !tightEnough(q, runs)) return;

    // Bands, so a contiguous match never loses to a scattered one on tie-breaks.
    let score = contiguous ? 2000 : 1000;
    if (isBoundary(text, start)) score += 200;
    score -= runs * 20;
    score -= span;
    score -= start;
    scored.push({ item, ranges, score });
  });

  scored.sort((a, b) => b.score - a.score);
  return scored.map(({ item, ranges }) => ({ item, ranges }));
}
