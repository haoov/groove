import { useEffect, useRef, useMemo } from 'react';
import {
  EditorView, Decoration, DecorationSet, GutterMarker,
  gutter, keymap, BlockInfo,
} from '@codemirror/view';
import { EditorState, StateField, StateEffect, RangeSetBuilder, Transaction } from '@codemirror/state';
import { syntaxHighlighting } from '@codemirror/language';
import { searchKeymap } from '@codemirror/search';
import { indentationMarkers } from '@replit/codemirror-indentation-markers';
import { vim } from '@replit/codemirror-vim';
import { setupVimSearch } from './cm/vimSetup';
import { viewBasics } from './cm/basics';
import { buildDocument, buildStaticDecos, type CMLineInfo } from './cm/diffDoc';
import { useStore } from '../shared/store';
import { cmLangFor } from './cmLang';
import { guessLang } from '../shared/lib/pure/lang';
import { catppuccinHighlight, cmChromeTheme } from './cm/theme';
import type { Hunk, Annotation, Mr, MrThread, BlameLine } from '../shared/ipc/ipc';
import type { AnnCtx, LineRange } from './useAnnotations';
import { CommentGutterMarker, LineNumGutterMarker, BlameMarker, FormWidget, InlineAnnotationsWidget } from './cm/gutters';
import { AnnotationPortals } from './cm/annotationPortals';
import { useCmHost } from './useCmHost';
import { useAnnotationSurface, extendDragAt, type Dyn } from './useAnnotationSurface';
import { gapsFor, type Gap } from './diffGaps';

const setDynEffect = StateEffect.define<Dyn>();

// ── Diff indicator gutter marker (far-left colored stripe) ───────────────────

class DiffIndicatorMarker extends GutterMarker {
  constructor(private type: 'add' | 'del' | 'ctx') { super(); }

  toDOM(): Node {
    const el = document.createElement('div');
    el.className = 'diff-indicator-bar';
    if (this.type === 'add') el.style.background = 'var(--gl-diff-add-edge)';
    else if (this.type === 'del') el.style.background = 'var(--gl-diff-del-edge)';
    return el;
  }

  eq(other: GutterMarker): boolean {
    return other instanceof DiffIndicatorMarker && other.type === this.type;
  }
}

// ── Dynamic Decorations ───────────────────────────────────────────────────────

function buildDynDecos(
  state: EditorState,
  lineMap: CMLineInfo[],
  dyn: Dyn,
  repoId: string,
  filePath: string,
): DecorationSet {
  const inRangeDeco   = Decoration.line({ class: 'diff-line-in-range' });
  const annotatedDeco = Decoration.line({ class: 'diff-line-annotated' });
  const activeRange = dyn.dragRange ?? dyn.sel;

  const annsByEndLine = new Map<number, Annotation[]>();
  for (const ann of dyn.fileAnnotations) {
    const group = annsByEndLine.get(ann.end_line);
    if (group) group.push(ann);
    else annsByEndLine.set(ann.end_line, [ann]);
  }

  const builder = new RangeSetBuilder<Decoration>();

  for (let n = 1; n <= lineMap.length; n++) {
    const info = lineMap[n - 1];
    const docLine = state.doc.line(n);
    const fn = info.fileLineNum;

    if (info.type !== 'del' && activeRange
      && activeRange.repoId === repoId && activeRange.filePath === filePath
      && fn >= activeRange.startLine && fn <= activeRange.endLine) {
      builder.add(docLine.from, docLine.from, inRangeDeco);
    } else if (dyn.annotatedLineNums.has(fn)) {
      builder.add(docLine.from, docLine.from, annotatedDeco);
    }

    if (info.type !== 'del') {
      // Inline annotations under their end line; hidden where the form is open.
      const lineAnns = annsByEndLine.get(fn);
      const annContainer = dyn.annContainers.get(fn);
      if (lineAnns && lineAnns.length > 0 && annContainer && fn !== dyn.anchorLine) {
        builder.add(docLine.to, docLine.to, Decoration.widget({
          widget: new InlineAnnotationsWidget(annContainer, lineAnns.map((a) => a.id).join(',')),
          block: true,
          side: 1,
        }));
      }

      // Comment form at the selection's end line.
      if (dyn.formEl && fn === dyn.anchorLine) {
        builder.add(docLine.to, docLine.to,
          Decoration.widget({ widget: new FormWidget(dyn.formEl), block: true, side: 1 }));
      }
    }
  }

  return builder.finish();
}

// ── CM Theme (diff-specifics; shared chrome comes from cmChromeTheme) ─────────

const cmTheme = EditorView.theme({
  '.cm-scroller': { overflow: 'visible !important' },
  '.cm-content': { padding: '0', color: 'var(--gl-text-color-default)' },
  '.cm-line': { padding: '0px 8px 0 6px', lineHeight: '25px', minHeight: '25px' },
  '.cm-diff-indicator-gutter': { width: '8px', minWidth: '6px' },
  '.cm-diff-indicator-gutter .cm-gutterElement': { padding: '0', width: '8px' },
  '.diff-indicator-bar': { width: '8px', minHeight: '25px', height: '100%' },
  '.cm-gutterElement': {
    padding: '0',
    lineHeight: '25px',
    fontSize: 'var(--gl-font-size-sm)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  '.cm-activeLine': { background: 'rgba(140,170,238,0.07)' },
  '.cm-activeLineGutter': { background: 'rgba(140,170,238,0.07)' },
});

// Applied when vim is off.
const hideCaretTheme = EditorView.theme({
  '.cm-content': { caretColor: 'transparent' },
  '.cm-cursor, .cm-cursorLayer': { display: 'none !important' },
});

// Applied when vim is on; the block caret is styled in editor.css.
const vimCaretTheme = EditorView.theme({
  '.cm-content': { caretColor: 'var(--gl-text-color-default)' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--gl-text-color-default)', borderLeftWidth: '2px' },
});

// The vim-dependent extensions, bundled for one compartment.
// Vim needs `editable` true for movement; `readOnly` stays on.
function vimExtensions(on: boolean) {
  return on
    ? [vim(), EditorView.editable.of(true), vimCaretTheme]
    : [EditorView.editable.of(false), hideCaretTheme];
}

// ── Props ─────────────────────────────────────────────────────────────────────

export interface FileDiffEditorProps {
  hunks: Hunk[];
  filePath: string;
  repoId: string;
  ann: AnnCtx;
  sel: LineRange | null;
  dragRange: LineRange | null;
  fileAnnotations: Annotation[];
  threads: MrThread[];
  mr: Mr | null;
  /** Changes when a real open asks this editor to take focus; undefined for inactive panes. */
  focusSignal?: number;
  /** True while this tab is a transient preview; suppresses auto-focus. */
  isPreview?: boolean;
  /** False for historical diffs: no comment gutter, drag select or inline form. Default true. */
  allowAnnotations?: boolean;
  /** Fills a gap with the file's real lines. Omit to render plain hunk separators. */
  onExpandGap?: (gap: Gap, whole: boolean) => void;
  /** The file's line count; decides whether a gap follows the last hunk. */
  fileLineCount?: number;
  /** Per-line authorship, indexed by new-side line number. Absent: gutter off. */
  blame?: BlameLine[];
  onOpenCommit?: (sha: string) => void;
}

// ── Component ─────────────────────────────────────────────────────────────────

/** Read when the gutter paints. */
const nowSeconds = () => Math.floor(Date.now() / 1000);

export function FileDiffEditor({
  hunks, filePath, repoId, ann, sel, dragRange, fileAnnotations, threads, mr, focusSignal, isPreview,
  allowAnnotations = true, onExpandGap, fileLineCount, blame, onOpenCommit,
}: FileDiffEditorProps) {
  const vimMode = useStore((s) => s.vimMode);
  useEffect(() => { setupVimSearch(); }, []);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const annRef = useRef(ann);
  annRef.current = ann;
  // Refs: a new callback identity must not rebuild the extensions.
  const expandRef = useRef(onExpandGap);
  expandRef.current = onExpandGap;
  const openCommitRef = useRef(onOpenCommit);
  openCommitRef.current = onOpenCommit;

  const { doc, lineMap, hunkFirstCMLines } = useMemo(() => buildDocument(hunks), [hunks]);
  const gaps = useMemo(() => gapsFor(hunks, fileLineCount), [hunks, fileLineCount]);

  const { sets, anchorLine, groups, containersRef, formRef, formEl, portalProps } =
    useAnnotationSurface({
      ann, sel: allowAnnotations ? sel : null, annotations: fileAnnotations, threads, mr,
      repoId, filePath, formClass: 'diff-inline-portal',
    });

  const extensions = useMemo(() => {
    const lang = cmLangFor(guessLang(filePath));
    const lm = lineMap;
    const hfcl = hunkFirstCMLines;

    const dynField = StateField.define<{ dyn: Dyn; decos: DecorationSet }>({
      create() {
        const empty: Dyn = {
          sel: null, dragRange: null,
          annotatedLineNums: new Set(), annStartNums: new Set(),
          threadNums: new Set(), unresolvedThreadNums: new Set(),
          formEl: null, anchorLine: null,
          fileAnnotations: [],
          annContainers: new Map(),
        };
        return { dyn: empty, decos: Decoration.none };
      },
      update(prev: { dyn: Dyn; decos: DecorationSet }, tr: Transaction) {
        for (const e of tr.effects) {
          if (e.is(setDynEffect)) {
            return { dyn: e.value, decos: buildDynDecos(tr.state, lm, e.value, repoId, filePath) };
          }
        }
        return prev;
      },
      provide: (f: StateField<{ dyn: Dyn; decos: DecorationSet }>) =>
        EditorView.decorations.from(f, (v: { dyn: Dyn; decos: DecorationSet }) => v.decos),
    });

    const staticField = StateField.define<DecorationSet>({
      create: (state: EditorState) => buildStaticDecos(
        state, hunks, lm, hfcl, gaps,
        expandRef.current ? (g, whole) => expandRef.current?.(g, whole) : null,
      ),
      update: (d: DecorationSet) => d,
      provide: (f: StateField<DecorationSet>) => EditorView.decorations.from(f),
    });

    const diffIndicatorGutter = gutter({
      class: 'cm-diff-indicator-gutter',
      lineMarker(view: EditorView, line: BlockInfo) {
        const cmLine = view.state.doc.lineAt(line.from).number;
        const info = lm[cmLine - 1];
        if (!info || info.type === 'ctx') return null;
        return new DiffIndicatorMarker(info.type);
      },
      initialSpacer: () => new DiffIndicatorMarker('ctx'),
    });

    const commentGutter = gutter({
      class: 'cm-comment-gutter',
      lineMarker(view: EditorView, line: BlockInfo) {
        const cmLine = view.state.doc.lineAt(line.from).number;
        const info = lm[cmLine - 1];
        if (!info) return null;
        // del lines: a marker for the background, no button.
        const fileLineNum = info.type !== 'del' ? info.fileLineNum : 0;
        const cls = info.type === 'add' ? 'diff-line-add' : info.type === 'del' ? 'diff-line-del' : '';
        return new CommentGutterMarker(fileLineNum, (n, e) => {
          annRef.current.beginDrag(repoId, filePath, n, e as unknown as React.MouseEvent);
        }, cls);
      },
      initialSpacer: () => new CommentGutterMarker(0, () => {}),
    });

    // Blame is keyed by the new-side line number; `del` lines get an empty cell.
    const blameByLine = new Map((blame ?? []).map((b) => [b.line, b]));
    const blameGutter = gutter({
      class: 'cm-blame-gutter',
      lineMarker(view: EditorView, line: BlockInfo) {
        const info = lm[view.state.doc.lineAt(line.from).number - 1];
        if (!info) return null;
        const cls = info.type === 'add' ? 'diff-line-add' : info.type === 'del' ? 'diff-line-del' : '';
        const b = info.type === 'del' ? null : blameByLine.get(info.fileLineNum) ?? null;
        return new BlameMarker(b, (sha) => openCommitRef.current?.(sha), nowSeconds(), cls);
      },
      initialSpacer: () => new BlameMarker(null, () => {}, nowSeconds()),
    });

    const lineNumGutter = gutter({
      class: 'cm-linenum-gutter',
      lineMarker(view: EditorView, line: BlockInfo) {
        const cmLine = view.state.doc.lineAt(line.from).number;
        const info = lm[cmLine - 1];
        if (!info) return null;
        const { dyn } = view.state.field(dynField);
        const fn = info.fileLineNum;
        const cls = info.type === 'add' ? 'diff-line-add' : info.type === 'del' ? 'diff-line-del' : '';
        return new LineNumGutterMarker(
          info.type !== 'del' && fn > 0 ? String(fn) : '',
          dyn.annStartNums.has(fn),
          dyn.threadNums.has(fn),
          dyn.unresolvedThreadNums.has(fn),
          cls,
        );
      },
      initialSpacer: () => new LineNumGutterMarker('', false, false, false),
    });

    return [
      EditorState.readOnly.of(true),
      cmChromeTheme,
      cmTheme,
      ...viewBasics(),
      staticField,
      dynField,
      diffIndicatorGutter,
      ...(allowAnnotations ? [commentGutter] : []),
      lineNumGutter,
      // Keep last of the gutters: extension order is gutter order.
      ...(blame ? [blameGutter] : []),
      keymap.of(searchKeymap),
      ...(lang ? [lang] : []),
      syntaxHighlighting(catppuccinHighlight),
      indentationMarkers({ colors: { dark: 'rgba(98,104,128,0.28)', activeDark: 'rgba(186,187,241,0.55)', light: 'rgba(98,104,128,0.28)', activeLight: 'rgba(186,187,241,0.55)' } }),
    ];
  // `lineMap` identity is a dep: identical text at shifted lines needs a rebuild. deps omit `repoId`.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filePath, lineMap, hunkFirstCMLines, hunks, gaps, blame, allowAnnotations]);

  useCmHost({
    containerRef, viewRef, doc, extensions, vimExt: vimExtensions, measureOnMount: true,
    canFocus: vimMode, focusSignal, isPreview,
  });

  // Push dynamic state into CM.
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: setDynEffect.of({
        ...sets, fileAnnotations, sel, dragRange, anchorLine,
        formEl: formRef.current, annContainers: containersRef.current,
      }),
    });
  }, [sets, fileAnnotations, sel, dragRange, anchorLine, formEl, groups, formRef, containersRef]);

  return (
    <div
      className="diff-cm-host"
      onClick={(e) => e.stopPropagation()}
      onMouseMove={(e) => {
        // A del line carries the new-side number of the line above it; 0 has no anchor.
        if (allowAnnotations && dragRange) {
          extendDragAt(viewRef.current, e, ann, repoId, filePath, (n) => lineMap[n - 1]?.fileLineNum ?? 0);
        }
      }}
    >
      <div ref={containerRef} className="diff-cm-editor" />
      <AnnotationPortals {...portalProps} />
    </div>
  );
}
