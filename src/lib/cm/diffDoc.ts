import { Decoration, WidgetType, type DecorationSet } from '@codemirror/view';
import { EditorState, RangeSetBuilder } from '@codemirror/state';
import { wordDiff } from '../word-diff';
import { GAP_STEP, type Gap } from '../diffGaps';
import type { Hunk } from '../../types/ipc';

/**
 * Turning a hunk list into a CodeMirror document.
 *
 * The diff editor renders ONE document made of every hunk's lines back to back, so
 * a CodeMirror line number is not a file line number: `lineMap` is the translation,
 * and everything that anchors to a file line (annotations, threads, blame, the
 * comment gutter) goes through it. Pure, so the mapping can be tested directly.
 */

/** What a document line came from. `fileLineNum` is the NEW-side number. */
export interface CMLineInfo {
  fileLineNum: number;
  type: 'add' | 'del' | 'ctx';
}

// ── Widgets ───────────────────────────────────────────────────────────────────

class HunkSepWidget extends WidgetType {
  toDOM(): HTMLElement {
    const el = document.createElement('div');
    el.className = 'diff-hunk-sep';
    return el;
  }
  eq(): boolean { return true; }
  // Must match the CSS box height (.diff-hunk-sep) so the gutter stays aligned.
  get estimatedHeight(): number { return 13; }
}

/** The clickable band over a gap: click adds `GAP_STEP` lines, Shift-click all. */
class GapWidget extends WidgetType {
  constructor(
    private gap: Gap,
    private onExpand: (gap: Gap, whole: boolean) => void,
  ) { super(); }

  toDOM(): HTMLElement {
    const count = this.gap.endLine - this.gap.startLine + 1;
    const el = document.createElement('div');
    el.className = 'diff-gap';
    const btn = document.createElement('button');
    btn.className = 'diff-gap-btn';
    btn.textContent = count <= GAP_STEP
      ? `⋯ ${count} hidden line${count === 1 ? '' : 's'} ⋯`
      : `⋯ ${count} hidden lines — click for ${GAP_STEP}, shift-click for all ⋯`;
    btn.title = `Lines ${this.gap.startLine}–${this.gap.endLine}`;
    btn.onclick = (e) => { e.stopPropagation(); this.onExpand(this.gap, e.shiftKey); };
    el.appendChild(btn);
    return el;
  }

  eq(other: WidgetType): boolean {
    return other instanceof GapWidget
      && other.gap.startLine === this.gap.startLine
      && other.gap.endLine === this.gap.endLine;
  }

  // Must match the CSS box height (.diff-gap) so the gutter stays aligned.
  get estimatedHeight(): number { return 20; }
}

// ── Document Builder ──────────────────────────────────────────────────────────

export function buildDocument(hunks: Hunk[]): {
  doc: string;
  lineMap: CMLineInfo[];
  hunkFirstCMLines: number[];
} {
  const parts: string[] = [];
  const lineMap: CMLineInfo[] = [];
  const hunkFirstCMLines: number[] = [];
  for (const hunk of hunks) {
    hunkFirstCMLines.push(lineMap.length + 1);
    for (const line of hunk.lines) {
      parts.push(line.content);
      lineMap.push({ fileLineNum: line.num, type: line.type });
    }
  }
  return { doc: parts.join('\n'), lineMap, hunkFirstCMLines };
}

// ── Static Decorations ────────────────────────────────────────────────────────

export function buildStaticDecos(
  state: EditorState,
  hunks: Hunk[],
  lineMap: CMLineInfo[],
  hunkFirstCMLines: number[],
  gaps: Gap[],
  onExpand: ((gap: Gap, whole: boolean) => void) | null,
): DecorationSet {
  // Pre-compute word-diff ranges (del/add pairs within each hunk)
  const wordRanges = new Map<number, { ranges: [number, number][] }>();
  for (let hi = 0; hi < hunks.length; hi++) {
    const { lines } = hunks[hi];
    let i = 0;
    while (i < lines.length) {
      if (lines[i].type !== 'del') { i++; continue; }
      const delStart = i;
      while (i < lines.length && lines[i].type === 'del') i++;
      const addStart = i;
      while (i < lines.length && lines[i].type === 'add') i++;
      const pairs = Math.min(i - addStart, addStart - delStart);
      for (let p = 0; p < pairs; p++) {
        const { delRanges, addRanges } = wordDiff(lines[delStart + p].content, lines[addStart + p].content);
        const delCM = hunkFirstCMLines[hi] + delStart + p;
        const addCM = hunkFirstCMLines[hi] + addStart + p;
        if (delRanges.length) wordRanges.set(delCM, { ranges: delRanges });
        if (addRanges.length) wordRanges.set(addCM, { ranges: addRanges });
      }
    }
  }

  const builder = new RangeSetBuilder<Decoration>();
  const sepWidget = new HunkSepWidget();
  // Gaps are keyed by the hunk they sit above; `hunks.length` is the trailing one.
  const gapAbove = new Map(gaps.map((g) => [g.beforeHunk, g]));

  for (let n = 1; n <= lineMap.length; n++) {
    const info = lineMap[n - 1];
    const docLine = state.doc.line(n);

    // A band above every hunk that has hidden lines before it, plus the plain
    // separator between hunks when expansion is unavailable.
    const hunkIdx = hunkFirstCMLines.indexOf(n);
    if (hunkIdx >= 0) {
      const gap = gapAbove.get(hunkIdx);
      if (gap && onExpand) {
        builder.add(docLine.from, docLine.from,
          Decoration.widget({ widget: new GapWidget(gap, onExpand), block: true, side: -1 }));
      } else if (hunkIdx > 0) {
        builder.add(docLine.from, docLine.from,
          Decoration.widget({ widget: sepWidget, block: true, side: -1 }));
      }
    }

    // Line background
    if (info.type === 'add') {
      builder.add(docLine.from, docLine.from, Decoration.line({ class: 'diff-line-add' }));
    } else if (info.type === 'del') {
      builder.add(docLine.from, docLine.from, Decoration.line({ class: 'diff-line-del' }));
    }

    // Word-diff marks
    const wr = wordRanges.get(n);
    if (wr) {
      for (const [start, end] of wr.ranges) {
        const from = docLine.from + start;
        const to = docLine.from + end;
        if (from < to && to <= docLine.to) {
          builder.add(from, to, Decoration.mark({ class: 'diff-word-mark' }));
        }
      }
    }
  }

  // Trailing gap: below the last line, so it needs `side: 1`.
  const tail = gapAbove.get(hunks.length);
  if (tail && onExpand) {
    builder.add(state.doc.length, state.doc.length,
      Decoration.widget({ widget: new GapWidget(tail, onExpand), block: true, side: 1 }));
  }
  return builder.finish();
}

