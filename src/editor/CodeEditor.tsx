import { useEffect, useMemo, useRef } from 'react';
import {
  EditorView, gutter, keymap, Decoration, DecorationSet, BlockInfo,
  ViewPlugin, ViewUpdate,
} from '@codemirror/view';
import { EditorState, StateField, StateEffect, RangeSetBuilder, Compartment } from '@codemirror/state';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { syntaxHighlighting } from '@codemirror/language';
import { searchKeymap } from '@codemirror/search';
import { indentationMarkers } from '@replit/codemirror-indentation-markers';
import { vim, Vim } from '@replit/codemirror-vim';
import { setupVimSearch } from './cm/vimSetup';
import { viewBasics } from './cm/basics';
import { invoke } from '../shared/ipc/invoke';
import { errorText } from '../shared/lib/pure/appError';
import { useStore, type GrepHighlight } from '../shared/store';
import { cmLangFor } from './cmLang';
import { catppuccinHighlight, cmChromeTheme, editorTheme } from './cm/theme';
import type { Annotation, MrThread, Mr, BlameLine } from '../shared/ipc/ipc';
import type { AnnCtx } from './useAnnotations';
import { CommentGutterMarker, LineNumGutterMarker, BlameMarker, FormWidget, InlineAnnotationsWidget } from './cm/gutters';
import { AnnotationPortals } from './cm/annotationPortals';
import { useCmHost } from './useCmHost';
import { useAnnotationSurface, extendDragAt, type Dyn } from './useAnnotationSurface';

// ── Dynamic state (gutter indicators, in-range highlight, inline widgets) ─────

const emptyDyn: Dyn = {
  annStartNums: new Set(), annotatedLineNums: new Set(), threadNums: new Set(),
  unresolvedThreadNums: new Set(), fileAnnotations: [], sel: null, anchorLine: null, formEl: null,
  annContainers: new Map(),
};

const setEditorDyn = StateEffect.define<Dyn>();

function buildDecos(state: EditorState, dyn: Dyn): DecorationSet {
  const b = new RangeSetBuilder<Decoration>();
  const lines = state.doc.lines;
  const anchor = dyn.anchorLine;

  const annsByEndLine = new Map<number, Annotation[]>();
  for (const ann of dyn.fileAnnotations) {
    const g = annsByEndLine.get(ann.end_line);
    if (g) g.push(ann); else annsByEndLine.set(ann.end_line, [ann]);
  }

  for (let n = 1; n <= lines; n++) {
    const line = state.doc.line(n);
    const inRange = dyn.sel && n >= dyn.sel.startLine && n <= dyn.sel.endLine;
    if (inRange) {
      b.add(line.from, line.from, Decoration.line({ class: 'diff-line-in-range' }));
    } else if (dyn.annotatedLineNums.has(n)) {
      b.add(line.from, line.from, Decoration.line({ class: 'diff-line-annotated' }));
    }
    // Inline annotations under their end line; hidden where the form is open.
    const lineAnns = annsByEndLine.get(n);
    const annContainer = dyn.annContainers.get(n);
    if (lineAnns && lineAnns.length > 0 && annContainer && n !== anchor) {
      b.add(line.to, line.to, Decoration.widget({
        widget: new InlineAnnotationsWidget(annContainer, lineAnns.map((a) => a.id).join(',')),
        block: true,
        side: 1,
      }));
    }
    // Comment form at the selection's end line.
    if (dyn.formEl && n === anchor) {
      b.add(line.to, line.to, Decoration.widget({ widget: new FormWidget(dyn.formEl), block: true, side: 1 }));
    }
  }
  return b.finish();
}

// ── Grep match highlight (content-search preview) ─────────────────────────────

const setGrepQuery = StateEffect.define<GrepHighlight | null>();

const grepQueryField = StateField.define<GrepHighlight | null>({
  create() { return null; },
  update(prev, tr) {
    for (const e of tr.effects) if (e.is(setGrepQuery)) return e.value;
    return prev;
  },
});

/** Marks the hits on one line: the row the search cursor is on. */
function buildGrepDecos(view: EditorView, h: GrepHighlight): DecorationSet {
  const b = new RangeSetBuilder<Decoration>();
  const needle = h.query.toLowerCase();
  const doc = view.state.doc;
  if (needle.length < 2 || h.line < 1 || h.line > doc.lines) return b.finish();
  const line = doc.line(h.line);
  const hay = line.text.toLowerCase();
  for (let i = hay.indexOf(needle); i !== -1; i = hay.indexOf(needle, i + needle.length)) {
    b.add(line.from + i, line.from + i + needle.length, Decoration.mark({ class: 'cm-grep-match' }));
  }
  return b.finish();
}

const grepPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      const q = view.state.field(grepQueryField);
      this.decorations = q ? buildGrepDecos(view, q) : Decoration.none;
    }
    update(update: ViewUpdate) {
      const q = update.state.field(grepQueryField);
      const queryChanged = update.transactions.some((tr) => tr.effects.some((e) => e.is(setGrepQuery)));
      if (queryChanged || update.docChanged) {
        this.decorations = q ? buildGrepDecos(update.view, q) : Decoration.none;
      }
    }
  },
  { decorations: (v) => v.decorations },
);

const dynField = StateField.define<{ dyn: Dyn; decos: DecorationSet }>({
  create() { return { dyn: emptyDyn, decos: Decoration.none }; },
  update(prev, tr) {
    for (const e of tr.effects) {
      if (e.is(setEditorDyn)) return { dyn: e.value, decos: buildDecos(tr.state, e.value) };
    }
    if (tr.docChanged && prev.decos !== Decoration.none) {
      return { dyn: prev.dyn, decos: prev.decos.map(tr.changes) };
    }
    return prev;
  },
  provide: (f) => EditorView.decorations.from(f, (v) => v.decos),
});

const vimExt = (on: boolean) => (on ? vim() : []);

// ── Props ─────────────────────────────────────────────────────────────────────

export interface CodeEditorProps {
  worktreePath: string;
  filePath: string;
  repoId: string;
  languageId: string;
  initialCursorLine: number;
  initialCursorCol: number;
  /** Open annotations for this file. */
  annotations: Annotation[];
  /** MR threads positioned in this file. */
  threads: MrThread[];
  /** The MR for this worktree. */
  mr: Mr | null;
  /** Shared annotation context. */
  ann: AnnCtx;
  onModifiedChange: (modified: boolean) => void;
  onPersistCursor: (line: number, col: number, scrollTop: number) => void;
  onSaveContent: (content: string) => Promise<void>;
  /** Hands the parent a save() to call. */
  registerSave?: (fn: () => void) => void;
  /** Changes when a real open asks this editor to take focus; undefined for inactive panes. */
  focusSignal?: number;
  /** True while this tab is a transient preview; suppresses auto-focus. */
  isPreview?: boolean;
  /** Per-line authorship, in file order. Absent: the blame gutter is off. */
  blame?: BlameLine[];
  onOpenCommit?: (sha: string) => void;
}

// ── Component ─────────────────────────────────────────────────────────────────

export function CodeEditor(props: CodeEditorProps) {
  const { ann, repoId, filePath } = props;
  const grepHighlight = useStore((s) => s.grepHighlight);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const blameCompartment = useRef(new Compartment());
  const loadingRef = useRef(false);
  const persistTimer = useRef<number | null>(null);

  const propsRef = useRef(props);
  propsRef.current = props;
  const annRef = useRef(ann);
  annRef.current = ann;

  const { sets, thisFileSel, anchorLine, groups, containersRef, formRef, formEl, portalProps } =
    useAnnotationSurface({
      ann, sel: ann.sel, annotations: props.annotations, threads: props.threads, mr: props.mr,
      repoId, filePath, formClass: 'editor-inline-portal',
    });
  const setsRef = useRef(sets);
  setsRef.current = sets;

  const doSave = async () => {
    const view = viewRef.current;
    if (!view) return;
    try {
      await propsRef.current.onSaveContent(view.state.doc.toString());
      propsRef.current.onModifiedChange(false);
    } catch (e) {
      useStore.getState().notify({ kind: 'error', source: 'files', title: 'Save failed', detail: errorText(e) });
    }
  };
  const saveRef = useRef(doSave);
  saveRef.current = doSave;

  // Every `EditorState.create` must include this compartment; `setState` drops any left out.
  const blameExtension = (blame?: BlameLine[]) => {
    if (!blame) return [];
    const byLine = new Map(blame.map((b) => [b.line, b]));
    // Read when the gutter paints.
    const now = () => Math.floor(Date.now() / 1000);
    return gutter({
      class: 'cm-blame-gutter',
      lineMarker: (view: EditorView, line: BlockInfo) => new BlameMarker(
        byLine.get(view.state.doc.lineAt(line.from).number) ?? null,
        (sha) => propsRef.current.onOpenCommit?.(sha),
        now(),
      ),
      initialSpacer: () => new BlameMarker(null, () => {}, now()),
    });
  };

  const schedulePersist = () => {
    if (persistTimer.current !== null) clearTimeout(persistTimer.current);
    persistTimer.current = window.setTimeout(() => {
      persistTimer.current = null;
      const view = viewRef.current;
      if (!view) return;
      const head = view.state.selection.main.head;
      const line = view.state.doc.lineAt(head);
      propsRef.current.onPersistCursor(line.number, head - line.from + 1, view.scrollDOM.scrollTop);
    }, 400);
  };

  // Stable extensions; handlers read refs.
  const extensions = useMemo(() => {
    // Gutter order mirrors the diff editor: comment button, then line numbers.
    const commentGutter = gutter({
      class: 'cm-comment-gutter',
      lineMarker(view: EditorView, line: BlockInfo) {
        const ln = view.state.doc.lineAt(line.from).number;
        return new CommentGutterMarker(ln, (n, e) => {
          annRef.current.beginDrag(propsRef.current.repoId, propsRef.current.filePath, n, e as unknown as React.MouseEvent);
        });
      },
      initialSpacer: () => new CommentGutterMarker(0, () => {}),
    });
    const lineNumGutter = gutter({
      class: 'cm-linenum-gutter',
      lineMarker(view: EditorView, line: BlockInfo) {
        const ln = view.state.doc.lineAt(line.from).number;
        const { dyn } = view.state.field(dynField);
        return new LineNumGutterMarker(
          String(ln),
          dyn.annStartNums.has(ln),
          dyn.threadNums.has(ln),
          dyn.unresolvedThreadNums.has(ln),
        );
      },
      lineMarkerChange: (update) =>
        update.transactions.some((tr) => tr.effects.some((e) => e.is(setEditorDyn))),
      initialSpacer: () => new LineNumGutterMarker('', false, false, false),
    });
    return [
      history(),
      commentGutter,
      lineNumGutter,
      dynField,
      grepQueryField,
      grepPlugin,
      ...viewBasics(),
      keymap.of([
        { key: 'Mod-s', run: () => { saveRef.current(); return true; }, preventDefault: true },
        indentWithTab,
        ...searchKeymap,
        ...defaultKeymap,
        ...historyKeymap,
      ]),
      syntaxHighlighting(catppuccinHighlight),
      indentationMarkers({ colors: { dark: 'rgba(98,104,128,0.28)', activeDark: 'rgba(186,187,241,0.55)', light: 'rgba(98,104,128,0.28)', activeLight: 'rgba(186,187,241,0.55)' } }),
      cmChromeTheme,
      editorTheme,
      EditorView.updateListener.of((update) => {
        if (loadingRef.current) return;
        if (update.docChanged) propsRef.current.onModifiedChange(true);
        if (update.docChanged || update.selectionSet || update.geometryChanged) schedulePersist();
      }),
      // Keep after the gutters above: extension order is gutter order.
      blameCompartment.current.of(blameExtension(propsRef.current.blame)),
    ];
  // deps stay empty: a rebuild recreates the view and drops the buffer.
  }, []);

  // `:w` / `:wq` save the buffer. Vim's ex-command map is global.
  useEffect(() => {
    Vim.defineEx('write', 'w', () => { saveRef.current(); });
    Vim.defineEx('wq', 'wq', () => { saveRef.current(); });
    setupVimSearch();
  }, []);

  const { vimCompartment } = useCmHost({
    containerRef,
    viewRef,
    doc: '',
    extensions,
    vimExt,
    focusSignal: props.focusSignal,
    isPreview: props.isPreview,
    onDestroy: () => { if (persistTimer.current !== null) clearTimeout(persistTimer.current); },
  });

  // Toggle blame in place.
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: blameCompartment.current.reconfigure(blameExtension(props.blame)),
    });
  }, [props.blame]);

  const registerSave = props.registerSave;
  useEffect(() => {
    registerSave?.(() => { saveRef.current(); });
  }, [registerSave]);

  // Load the file and rebuild the state with its language.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    let cancelled = false;
    loadingRef.current = true;
    invoke<string>('read_file', { worktreePath: props.worktreePath, filePath: props.filePath })
      .then((content) => {
        if (cancelled || !viewRef.current) return;
        const lang = cmLangFor(props.languageId);
        // The vim compartment must be re-declared; `setState` drops any left out.
        const base = [vimCompartment.of(vimExt(useStore.getState().vimMode)), ...extensions];
        const state = EditorState.create({
          doc: content,
          extensions: lang ? [...base, lang] : base,
        });
        view.setState(state);
        // Restore cursor and scroll; re-push the dynamic state `setState` reset.
        const lineNo = Math.min(Math.max(1, props.initialCursorLine || 1), state.doc.lines);
        const lineObj = state.doc.line(lineNo);
        const pos = Math.min(lineObj.from + Math.max(0, (props.initialCursorCol || 1) - 1), lineObj.to);
        const effects: StateEffect<any>[] = [
          setEditorDyn.of({ ...setsRef.current, fileAnnotations: propsRef.current.annotations, sel: null, anchorLine: null, formEl: null, annContainers: containersRef.current }),
          setGrepQuery.of(useStore.getState().grepHighlight),
        ];
        if (props.initialCursorLine > 0) effects.push(EditorView.scrollIntoView(pos, { y: 'center' }));
        view.dispatch({ selection: { anchor: pos }, effects });
        loadingRef.current = false;
        propsRef.current.onModifiedChange(false);
      })
      .catch((e) => {
        if (cancelled || !viewRef.current) return;
        view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: `// Error loading file: ${errorText(e)}` } });
        loadingRef.current = false;
      });
    return () => { cancelled = true; };
  // deps omit the cursor and language props: a change there would reload the file.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.worktreePath, props.filePath, extensions]);

  // Push the content-search query into the highlight and scroll to the selected match.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({ effects: setGrepQuery.of(grepHighlight) });
    const line = grepHighlight?.line ?? 0;
    if (line > 0 && line <= view.state.doc.lines) {
      view.dispatch({ effects: EditorView.scrollIntoView(view.state.doc.line(line).from, { y: 'center' }) });
    }
  }, [grepHighlight]);

  // Push dynamic state into CM whenever annotations/threads or the selection change.
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: setEditorDyn.of({
        ...sets, fileAnnotations: props.annotations, sel: thisFileSel, anchorLine,
        formEl: formRef.current, annContainers: containersRef.current,
      }),
    });
  }, [sets, props.annotations, thisFileSel, anchorLine, formEl, groups, formRef, containersRef]);

  return (
    <div
      className="code-editor-host"
      onMouseMove={(e) => {
        if (ann.dragRange) extendDragAt(viewRef.current, e, ann, repoId, filePath, (n) => n);
      }}
    >
      <div ref={containerRef} className="code-editor" />
      <AnnotationPortals {...portalProps} />
    </div>
  );
}
