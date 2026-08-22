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
