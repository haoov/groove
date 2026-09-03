import { useEffect, useRef, useMemo } from 'react';
import {
  EditorView, Decoration, DecorationSet, GutterMarker,
  gutter, keymap, BlockInfo,
} from '@codemirror/view';
import { EditorState, StateField, StateEffect, RangeSetBuilder, Transaction, Compartment } from '@codemirror/state';
import { syntaxHighlighting } from '@codemirror/language';
import { searchKeymap } from '@codemirror/search';
import { indentationMarkers } from '@replit/codemirror-indentation-markers';
import { vim } from '@replit/codemirror-vim';
import { setupVimSearch } from './cm/vimSetup';
import { viewBasics } from './cm/basics';
import { buildDocument, buildStaticDecos, type CMLineInfo } from './cm/diffDoc';
import { useStore } from '../shared/store';
import { cmLangFor } from './cmLang';
import { guessLang } from '../shared/lib/lang';
import { catppuccinHighlight, cmChromeTheme } from './cm/theme';
import type { Hunk, Annotation, Mr, MrThread, BlameLine } from '../shared/ipc/ipc';
import type { AnnCtx, LineRange } from './useAnnotations';
import { CommentGutterMarker, LineNumGutterMarker, BlameMarker, FormWidget, InlineAnnotationsWidget } from './cm/gutters';
import { deriveAnnotationSets } from './cm/annotationSets';
import { AnnotationPortals, useAnnotationPortals } from './cm/annotationPortals';
import { gapsFor, type Gap } from '../shared/lib/diffGaps';

// ── Types ─────────────────────────────────────────────────────────────────────

interface DynState {
  sel: LineRange | null;
  dragRange: LineRange | null;
  annotatedLineNums: Set<number>;
  annStartNums: Set<number>;
  threadNums: Set<number>;
  unresolvedThreadNums: Set<number>;
  inlineContainer: HTMLDivElement | null;
  inlineAnchorNum: number | null;
  fileAnnotations: Annotation[];
  /** Portal target per annotated end-line for the always-visible notes. */
  annContainers: Map<number, HTMLDivElement>;
}

const setDynEffect = StateEffect.define<DynState>();

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
  dyn: DynState,
  repoId: string,
  filePath: string,
): DecorationSet {
  const inRangeDeco   = Decoration.line({ class: 'diff-line-in-range' });
  const annotatedDeco = Decoration.line({ class: 'diff-line-annotated' });
  const activeRange = dyn.dragRange ?? dyn.sel;

  // Group annotations by end_line for permanent inline widget placement.
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
      // Permanent annotation widget — always visible below each annotation's end line.
      // Suppressed when the comment form is open at this same line (form already shows them).
      const lineAnns = annsByEndLine.get(fn);
      const annContainer = dyn.annContainers.get(fn);
      if (lineAnns && lineAnns.length > 0 && annContainer && fn !== dyn.inlineAnchorNum) {
        builder.add(docLine.to, docLine.to, Decoration.widget({
          widget: new InlineAnnotationsWidget(annContainer, lineAnns.map((a) => a.id).join(',')),
          block: true,
          side: 1,
        }));
      }

      // Comment form widget — opens when the user selects/drags on the gutter.
      if (dyn.inlineContainer && fn === dyn.inlineAnchorNum) {
        builder.add(docLine.to, docLine.to,
          Decoration.widget({ widget: new FormWidget(dyn.inlineContainer), block: true, side: 1 }));
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
  // Diff indicator gutter — narrow colored stripe at the far left
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

// Hides the caret — applied only when vim navigation is OFF (a plain readonly diff).
const hideCaretTheme = EditorView.theme({
  '.cm-content': { caretColor: 'transparent' },
  '.cm-cursor, .cm-cursorLayer': { display: 'none !important' },
});

// Visible cursor for vim navigation in the diff (block caret styled in editor.css).
const vimCaretTheme = EditorView.theme({
  '.cm-content': { caretColor: 'var(--gl-text-color-default)' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--gl-text-color-default)', borderLeftWidth: '2px' },
});

// The vim-dependent extensions, bundled so they can live in a compartment and be
// toggled in place. Vim navigation needs the view `editable` (movement + search,
// no edits — readOnly stays on) and a visible caret; without vim it's a plain,
// non-focusable, caret-less readonly view.
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
  /** Changes when a real open/commit asks this editor to take focus (so vim nav
   *  works); undefined for inactive panes. A preview open never bumps it. */
  focusSignal?: number;
  /** True while this tab is a transient preview — suppresses auto-focus. */
  isPreview?: boolean;
  /** False for historical diffs (commit view): annotations anchor to working-tree
   *  line numbers, which a past commit doesn't map to — the comment gutter, drag
   *  select, and inline form are omitted. Default true. */
  allowAnnotations?: boolean;
  /** Fills a gap with the file's real lines (see `useDiffExpand`). Omit to render
   *  plain, non-clickable hunk separators. */
  onExpandGap?: (gap: Gap, whole: boolean) => void;
  /** The file's line count, which decides whether a gap follows the last hunk. */
  fileLineCount?: number;
  /** Per-line authorship, indexed by new-side line number. Absent = gutter off. */
  blame?: BlameLine[];
  onOpenCommit?: (sha: string) => void;
}

// ── Component ─────────────────────────────────────────────────────────────────

/** Blame age is measured when the gutter paints, never during render. */
const nowSeconds = () => Math.floor(Date.now() / 1000);

export function FileDiffEditor({
  hunks, filePath, repoId, ann, sel, dragRange, fileAnnotations, threads, mr, focusSignal, isPreview,
  allowAnnotations = true, onExpandGap, fileLineCount, blame, onOpenCommit,
}: FileDiffEditorProps) {
  const vimMode = useStore((s) => s.vimMode);
  useEffect(() => { setupVimSearch(); }, []);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // Toggling vim reconfigures this compartment in place so the readonly diff
  // keeps its selection/scroll instead of the view being recreated.
  const vimCompartment = useRef(new Compartment());
  const annRef = useRef(ann);
  annRef.current = ann;
  // Read through refs so a new callback identity never rebuilds the extensions.
  const expandRef = useRef(onExpandGap);
  expandRef.current = onExpandGap;
  const openCommitRef = useRef(onOpenCommit);
  openCommitRef.current = onOpenCommit;

  const { doc, lineMap, hunkFirstCMLines } = useMemo(() => buildDocument(hunks), [hunks]);
  const gaps = useMemo(() => gapsFor(hunks, fileLineCount), [hunks, fileLineCount]);

  // Annotation/thread sets derived from props
  const { annotatedLineNums, annStartNums, threadNums, unresolvedThreadNums } = useMemo(
    () => deriveAnnotationSets(fileAnnotations, threads, filePath),
    [fileAnnotations, threads, filePath],
  );

  const thisFileSel =
    allowAnnotations && sel?.repoId === repoId && sel?.filePath === filePath ? sel : null;
  const inlineAnchorNum = thisFileSel?.endLine ?? null;

  const { groups: annGroups, containersRef: annContainersRef, formRef: portalContainerRef, formEl: portalContainer } =
    useAnnotationPortals(fileAnnotations, inlineAnchorNum, 'diff-inline-portal');

  // Build CM extensions (stable per filePath/doc change)
  const extensions = useMemo(() => {
    const lang = cmLangFor(guessLang(filePath));
    const lm = lineMap;
    const hfcl = hunkFirstCMLines;

    const dynField = StateField.define<{ dyn: DynState; decos: DecorationSet }>({
      create() {
        const empty: DynState = {
          sel: null, dragRange: null,
          annotatedLineNums: new Set(), annStartNums: new Set(),
          threadNums: new Set(), unresolvedThreadNums: new Set(),
          inlineContainer: null, inlineAnchorNum: null,
          fileAnnotations: [],
          annContainers: new Map(),
        };
        return { dyn: empty, decos: Decoration.none };
      },
      update(prev: { dyn: DynState; decos: DecorationSet }, tr: Transaction) {
        for (const e of tr.effects) {
          if (e.is(setDynEffect)) {
            return { dyn: e.value, decos: buildDynDecos(tr.state, lm, e.value, repoId, filePath) };
          }
        }
        return prev;
      },
      provide: (f: StateField<{ dyn: DynState; decos: DecorationSet }>) =>
        EditorView.decorations.from(f, (v: { dyn: DynState; decos: DecorationSet }) => v.decos),
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
        // del lines: return a marker (for background) but no comment button
        const fileLineNum = info.type !== 'del' ? info.fileLineNum : 0;
        const cls = info.type === 'add' ? 'diff-line-add' : info.type === 'del' ? 'diff-line-del' : '';
        return new CommentGutterMarker(fileLineNum, (n, e) => {
          annRef.current.beginDrag(repoId, filePath, n, e as unknown as React.MouseEvent);
        }, cls);
      },
      initialSpacer: () => new CommentGutterMarker(0, () => {}),
    });

    // Blame is keyed by the NEW-side line number, so `del` lines get an empty cell.
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
      // Vim (+ editable + caret) lives in a compartment so toggling it never
      // rebuilds these extensions / recreates the view (see the toggle effect).
      vimCompartment.current.of(vimExtensions(useStore.getState().vimMode)),
      EditorState.readOnly.of(true),
      cmChromeTheme,
      cmTheme,
      ...viewBasics(),
      staticField,
      dynField,
      diffIndicatorGutter,
      // Comment gutter only where annotations can anchor.
      ...(allowAnnotations ? [commentGutter] : []),
      lineNumGutter,
      // Registered only while blame is on, so it costs nothing when off. Last of
      // the gutters, which puts it next to the code as in the edit view.
      ...(blame ? [blameGutter] : []),
      keymap.of(searchKeymap),
      ...(lang ? [lang] : []),
      syntaxHighlighting(catppuccinHighlight),
      indentationMarkers({ colors: { dark: 'rgba(98,104,128,0.28)', activeDark: 'rgba(186,187,241,0.55)', light: 'rgba(98,104,128,0.28)', activeLight: 'rgba(186,187,241,0.55)' } }),
    ];
  // Depends on the buildDocument RESULT (lineMap/hunkFirstCMLines identity), not
  // just the doc string — identical text at shifted line numbers still needs a
  // fresh lineMap. Vim is compartmentalized, so it's intentionally not a dep.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filePath, lineMap, hunkFirstCMLines, hunks, gaps, blame, allowAnnotations]);

  // Mount / remount editor when doc or extensions change
  useEffect(() => {
    if (!containerRef.current) return;
    const view = new EditorView({
      state: EditorState.create({ doc, extensions }),
      parent: containerRef.current,
    });
    viewRef.current = view;
    // The editor is created the moment a file is expanded, so the container may
    // still have no height when CodeMirror first measures — it then renders a
    // viewport of almost nothing while the line decorations (the add/del
    // backgrounds) are already painted, which looks like coloured but empty lines.
    // Re-measure once the row has its real size.
    view.requestMeasure();
    return () => { view.destroy(); viewRef.current = null; };
  }, [doc, extensions]);

  // Toggle vim in place — reconfigure the compartment, never recreate the view.
  useEffect(() => {
    viewRef.current?.dispatch({ effects: vimCompartment.current.reconfigure(vimExtensions(vimMode)) });
  }, [vimMode]);

  // With vim nav on, the diff is focusable — focus it on a real open/commit so
  // h/j/k/l work. Suppressed for preview opens (search input keeps focus).
  useEffect(() => {
    if (focusSignal === undefined || isPreview || !vimMode) return;
    viewRef.current?.focus();
  }, [focusSignal, isPreview, vimMode]);

  // Push dynamic state into CM on every relevant change
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: setDynEffect.of({
        sel, dragRange,
        annotatedLineNums, annStartNums, threadNums, unresolvedThreadNums,
        inlineContainer: portalContainerRef.current,
        inlineAnchorNum,
        fileAnnotations,
        annContainers: annContainersRef.current,
      }),
    });
  }, [sel, dragRange, annotatedLineNums, annStartNums, threadNums, unresolvedThreadNums, inlineAnchorNum, fileAnnotations, annGroups,
      portalContainerRef, annContainersRef]);

  return (
    <div
      className="diff-cm-host"
      onClick={(e) => e.stopPropagation()}
      onMouseMove={(e) => {
        if (!allowAnnotations || !dragRange) return;
        const view = viewRef.current;
        if (!view) return;
        // Hosted on the wrapper, not the view: CM binds domEventHandlers to
        // contentDOM, which the gutter the drag starts in is outside of.
        const pos = view.posAtCoords({ x: e.clientX, y: e.clientY }, false);
        const info = lineMap[view.state.doc.lineAt(pos).number - 1];
        // A del line carries the new-side number of the line above it; 0 means
        // the file starts with a deletion, which no annotation can anchor to.
        if (info && info.fileLineNum > 0) ann.extendDrag(repoId, filePath, info.fileLineNum);
      }}
    >
      <div ref={containerRef} className="diff-cm-editor" />
      <AnnotationPortals
        groups={annGroups}
        containers={annContainersRef.current}
        formEl={portalContainer}
        sel={thisFileSel}
        annotations={fileAnnotations}
        threads={threads}
        mr={mr}
        ann={ann}
        repoId={repoId}
        filePath={filePath}
      />
    </div>
  );
}
