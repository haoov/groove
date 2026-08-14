import { GutterMarker, WidgetType } from '@codemirror/view';
import type { BlameLine } from '../../types/ipc';

// ── Inline SVG helpers ────────────────────────────────────────────────────────

function makeSvg(w: number, h: number, inner: string): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;
}

export const MSG_PLUS_SVG = makeSvg(22, 22,
  '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/><line x1="9" x2="15" y1="10" y2="10"/><line x1="12" x2="12" y1="7" y2="13"/>',
);
export const MSG_SVG = makeSvg(20, 20,
  '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>',
);

// ── Gutter markers (shared by the diff editor and the code editor) ────────────

/** Add-comment button. `fileLineNum === 0` renders an empty cell (e.g. diff del
 *  lines or spacers). `elementClass` carries the diff line-type background. */
export class CommentGutterMarker extends GutterMarker {
  elementClass: string;
  constructor(
    private fileLineNum: number,
    private cb: (n: number, e: MouseEvent) => void,
    elementClass = '',
  ) {
    super();
    this.elementClass = elementClass;
  }

  toDOM(): Node {
    const wrap = document.createElement('span');
    wrap.className = 'diff-comment-col-inner';
    if (this.fileLineNum === 0) return wrap;
    const btn = document.createElement('button');
    btn.className = 'diff-add-comment';
    btn.title = 'Comment — click for one line, drag to select a range';
    btn.innerHTML = MSG_PLUS_SVG;
    btn.addEventListener('mousedown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.cb(this.fileLineNum, e);
    });
    btn.addEventListener('click', (e) => e.stopPropagation());
    wrap.appendChild(btn);
    return wrap;
  }

  eq(other: GutterMarker): boolean {
    return other instanceof CommentGutterMarker
      && other.fileLineNum === this.fileLineNum
      && other.elementClass === this.elementClass;
  }
}

/** Line-number cell that shows the annotation icon + thread dot in place of the
 *  number when the line has annotations/threads. `numLabel` is the text shown
 *  otherwise (empty for diff deleted lines). */
export class LineNumGutterMarker extends GutterMarker {
  elementClass: string;
  constructor(
    private numLabel: string,
    private hasAnn: boolean,
    private hasThread: boolean,
    private unresolved: boolean,
    elementClass = '',
  ) {
    super();
    this.elementClass = elementClass;
  }

  toDOM(): Node {
    if (this.hasAnn || this.hasThread) {
      const wrap = document.createElement('span');
      wrap.className = 'diff-gutter-indicator active';
      if (this.hasAnn) {
        const span = document.createElement('span');
        span.className = 'diff-annotation-icon';
        span.innerHTML = MSG_SVG;
        wrap.appendChild(span);
      }
      if (this.hasThread) {
        const dot = document.createElement('span');
        dot.className = `diff-thread-dot ${this.unresolved ? 'unresolved' : 'resolved'}`;
        dot.title = 'thread';
        dot.textContent = '●';
        wrap.appendChild(dot);
      }
      return wrap;
    }
    const span = document.createElement('span');
    span.textContent = this.numLabel;
    return span;
  }

  eq(other: GutterMarker): boolean {
    return other instanceof LineNumGutterMarker
      && other.numLabel === this.numLabel
      && other.hasAnn === this.hasAnn
      && other.hasThread === this.hasThread
      && other.unresolved === this.unresolved
      && other.elementClass === this.elementClass;
  }
}

/** Compact relative age: what fits a gutter cell ("3d", "5mo", "2y"). */
function shortAge(epochSeconds: number, now: number): string {
  const s = Math.max(0, now - epochSeconds);
  const d = Math.floor(s / 86400);
  if (d < 1) return `${Math.max(1, Math.floor(s / 3600))}h`;
  if (d < 31) return `${d}d`;
  if (d < 365) return `${Math.floor(d / 30)}mo`;
  return `${Math.floor(d / 365)}y`;
}

/** First name only — a gutter cell has room for one word, not a full name. */
function shortAuthor(author: string): string {
  return author.split(' ')[0] ?? author;
}

/**
 * One line's blame: author and age, clicking opens that commit.
 *
 * Blame reads the file on disk, so an unsaved buffer attributes lines to whatever
 * git last saw there. `null` line data renders an empty cell (diff `del` lines and
 * the spacer).
 */
export class BlameMarker extends GutterMarker {
  elementClass: string;
  constructor(
    private info: BlameLine | null,
    private onOpenCommit: (sha: string) => void,
    private now: number,
    elementClass = '',
  ) {
    super();
    this.elementClass = elementClass;
  }

  toDOM(): Node {
    const el = document.createElement('span');
    el.className = 'cm-blame-cell';
    if (!this.info) return el;
    if (this.info.uncommitted) {
      el.classList.add('uncommitted');
      el.textContent = 'uncommitted';
      el.title = 'Not committed yet';
      return el;
    }
    el.textContent = `${shortAuthor(this.info.author)} ${shortAge(this.info.time, this.now)}`;
    el.title = `${this.info.summary}\n${this.info.short_sha} · ${this.info.author} · ${new Date(this.info.time * 1000).toLocaleString()}`;
    el.classList.add('clickable');
    el.addEventListener('mousedown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.onOpenCommit(this.info!.sha);
    });
    return el;
  }

  eq(other: GutterMarker): boolean {
    return other instanceof BlameMarker
      && other.info?.sha === this.info?.sha
      && other.info?.line === this.info?.line
      && other.elementClass === this.elementClass;
  }
}

// ── Block widgets ─────────────────────────────────────────────────────────────

/** Wraps the React portal container for the inline annotation form/panel. */
export class FormWidget extends WidgetType {
  constructor(public container: HTMLDivElement) { super(); }
  toDOM(): HTMLElement { return this.container; }
  eq(other: FormWidget): boolean { return this.container === other.container; }
  get estimatedHeight(): number { return 160; }
  ignoreEvent(): boolean { return true; }
}

/**
 * Permanent, always-visible annotations below a line.
 *
 * Like `FormWidget`, this is only a React portal target. Note bodies are
 * markdown (agents write code spans and lists), so they render through the
 * shared `<Markdown>` component — assembling the DOM here would mean a second,
 * diverging renderer.
 */
export class InlineAnnotationsWidget extends WidgetType {
  /** `ids` makes the widget compare unequal when the line's annotation set
   *  changes, so CodeMirror re-measures the block. */
  constructor(public container: HTMLDivElement, private ids: string) { super(); }

  toDOM(): HTMLElement { return this.container; }

  eq(other: WidgetType): boolean {
    return other instanceof InlineAnnotationsWidget
      && other.container === this.container
      && other.ids === this.ids;
  }

  /** -1 = unknown: markdown bodies vary in height, so let CM measure. */
  get estimatedHeight(): number { return -1; }
  ignoreEvent(): boolean { return true; }
}
