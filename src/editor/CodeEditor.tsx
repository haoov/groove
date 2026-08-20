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
import { useStore, type GrepHighlight } from '../shared/store';
import { cmLangFor } from './cmLang';
import { catppuccinHighlight, cmChromeTheme, editorTheme } from './cm/theme';
import type { Annotation, MrThread, Mr, BlameLine } from '../shared/ipc/ipc';
import type { AnnCtx } from './useAnnotations';
import { CommentGutterMarker, LineNumGutterMarker, BlameMarker, FormWidget, InlineAnnotationsWidget } from './cm/gutters';
import { deriveAnnotationSets } from './cm/annotationSets';
import { AnnotationPortals, useAnnotationPortals } from './cm/annotationPortals';

// ── Dynamic state (gutter indicators, in-range highlight, inline widgets) ─────

interface EditorDyn {
  annStartNums: Set<number>;     // lines that START an annotation (gutter icon)
  annotatedLineNums: Set<number>; // every line covered by an annotation (highlight)
  threadNums: Set<number>;
  unresolvedThreadNums: Set<number>;
  fileAnnotations: Annotation[];
  selStart: number | null;
  selEnd: number | null;
  portalEl: HTMLDivElement | null;
  /** Portal target per annotated end-line for the always-visible notes. */
  annContainers: Map<number, HTMLDivElement>;
}

const emptyDyn: EditorDyn = {
  annStartNums: new Set(), annotatedLineNums: new Set(), threadNums: new Set(),
  unresolvedThreadNums: new Set(), fileAnnotations: [], selStart: null, selEnd: null, portalEl: null,
  annContainers: new Map(),
};

const setEditorDyn = StateEffect.define<EditorDyn>();

function buildDecos(state: EditorState, dyn: EditorDyn): DecorationSet {
  const b = new RangeSetBuilder<Decoration>();
  const lines = state.doc.lines;
  const anchor = dyn.selEnd;

  // Group annotations by end line for permanent inline widgets.
  const annsByEndLine = new Map<number, Annotation[]>();
  for (const ann of dyn.fileAnnotations) {
    const g = annsByEndLine.get(ann.end_line);
    if (g) g.push(ann); else annsByEndLine.set(ann.end_line, [ann]);
  }

  for (let n = 1; n <= lines; n++) {
    const line = state.doc.line(n);
    const inRange = dyn.selStart && dyn.selEnd && n >= dyn.selStart && n <= dyn.selEnd;
    if (inRange) {
      b.add(line.from, line.from, Decoration.line({ class: 'diff-line-in-range' }));
    } else if (dyn.annotatedLineNums.has(n)) {
      b.add(line.from, line.from, Decoration.line({ class: 'diff-line-annotated' }));
    }
    // Permanent annotations under their end line (hidden while the form is open there).
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
    if (dyn.portalEl && n === anchor) {
      b.add(line.to, line.to, Decoration.widget({ widget: new FormWidget(dyn.portalEl), block: true, side: 1 }));
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

/**
 * Mark the hits on ONE line: the row the search cursor is on.
 *
 * Marking every occurrence in the file meant the match you had selected looked
 * exactly like the twenty you had not, which is the opposite of what walking the
 * result list is for. A line also needs no viewport scan.
 */
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
      // No viewport dependency: one line is marked, wherever it is.
      if (queryChanged || update.docChanged) {
        this.decorations = q ? buildGrepDecos(update.view, q) : Decoration.none;
      }
    }
  },
  { decorations: (v) => v.decorations },
);

const dynField = StateField.define<{ dyn: EditorDyn; decos: DecorationSet }>({
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
  /** The MR for this worktree (for inline thread replies). */
  mr: Mr | null;
  /** Shared annotation context — same one the diff editor uses. */
  ann: AnnCtx;
  onModifiedChange: (modified: boolean) => void;
  onPersistCursor: (line: number, col: number, scrollTop: number) => void;
  onSaveContent: (content: string) => Promise<void>;
  /** Hands the parent a save() it can call (Ctrl+S / Save button). */
  registerSave?: (fn: () => void) => void;
  /** Changes when a real (non-preview) open/commit asks this editor to take
   *  DOM focus. Undefined for inactive panes (so they never grab focus). */
  focusSignal?: number;
  /** True while this tab is a transient preview — suppresses auto-focus so the
   *  file-search input keeps focus as the user navigates results. */
  isPreview?: boolean;
  /** Per-line authorship, in file order. Absent = the blame gutter is off. */
  blame?: BlameLine[];
  onOpenCommit?: (sha: string) => void;
}

// ── Component ─────────────────────────────────────────────────────────────────

export function CodeEditor(props: CodeEditorProps) {
  const { ann, repoId, filePath } = props;
  const vimMode = useStore((s) => s.vimMode);
  const grepHighlight = useStore((s) => s.grepHighlight);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // Toggling vim reconfigures this compartment in place — the view is never
  // recreated and the buffer is never reloaded, so unsaved edits survive.
  const vimCompartment = useRef(new Compartment());
  // Same reason as vim: toggling blame must not reload the buffer or lose undo.
  const blameCompartment = useRef(new Compartment());
  const loadingRef = useRef(false);
  const persistTimer = useRef<number | null>(null);

  const propsRef = useRef(props);
  propsRef.current = props;
  const annRef = useRef(ann);
  annRef.current = ann;


  // The selection that belongs to this file (drives the inline form).
  const thisFileSel = ann.sel?.repoId === repoId && ann.sel?.filePath === filePath ? ann.sel : null;
  const anchorLine = thisFileSel?.endLine ?? null;

  // Indicator/highlight sets derived from annotations + threads.
  const dynSets = useMemo(
    () => deriveAnnotationSets(props.annotations, props.threads, filePath),
    [props.annotations, props.threads, filePath],
  );
  const dynSetsRef = useRef(dynSets);
  dynSetsRef.current = dynSets;

  const { groups: annGroups, containersRef: annContainersRef, formRef: portalContainerRef, formEl: portalContainer } =
    useAnnotationPortals(props.annotations, anchorLine, 'editor-inline-portal');

  const doSave = async () => {
    const view = viewRef.current;
    if (!view) return;
    try {
      await propsRef.current.onSaveContent(view.state.doc.toString());
      propsRef.current.onModifiedChange(false);
    } catch (e) {
      // Keep the modified flag set so the user knows the buffer is still dirty.
      useStore.getState().notify({ kind: 'error', source: 'files', title: `Save failed: ${e}` });
    }
  };
  const saveRef = useRef(doSave);
  saveRef.current = doSave;

  // Built on demand because every state this editor creates has to install it:
  // `view.setState` on a file load replaces the whole configuration, so a
  // compartment left out there is gone and later reconfigures do nothing.
  const blameExtension = (blame?: BlameLine[]) => {
    if (!blame) return [];
    const byLine = new Map(blame.map((b) => [b.line, b]));
    const now = Math.floor(Date.now() / 1000);
    return gutter({
      class: 'cm-blame-gutter',
      lineMarker: (view: EditorView, line: BlockInfo) => new BlameMarker(
        byLine.get(view.state.doc.lineAt(line.from).number) ?? null,
        (sha) => propsRef.current.onOpenCommit?.(sha),
        now,
      ),
      initialSpacer: () => new BlameMarker(null, () => {}, now),
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

  // Stable extensions (handlers read refs, so they never go stale).
  const baseExtensions = useMemo(() => {
    // Gutter order (left → right) mirrors the diff editor: add-comment button,
    // then the line numbers (with the annotation/thread indicator merged in).
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
    ];
  // Stable for the editor's lifetime — vim lives in a compartment (reconfigured
  // separately) so this never rebuilds and the view/buffer are never recreated.
  }, []);

  // Make `:w` / `:wq` save the buffer (registered once; Vim's ex-command map is global).
  useEffect(() => {
    Vim.defineEx('write', 'w', () => { saveRef.current(); });
    Vim.defineEx('wq', 'wq', () => { saveRef.current(); });
    setupVimSearch();
  }, []);

  // Mount the editor once. Vim goes in a compartment (see the toggle effect
  // below) so switching modes never recreates the view.
  useEffect(() => {
    if (!containerRef.current) return;
    const view = new EditorView({
      state: EditorState.create({
        doc: '',
        extensions: [
          vimCompartment.current.of(useStore.getState().vimMode ? vim() : []),
          ...baseExtensions,
          // After the gutters in baseExtensions, so blame sits next to the code.
          blameCompartment.current.of(blameExtension(propsRef.current.blame)),
        ],
      }),
      parent: containerRef.current,
    });
    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
      if (persistTimer.current !== null) clearTimeout(persistTimer.current);
    };
  }, [baseExtensions]);

  // Toggle vim in place — reconfigure the compartment, never recreate the view.
  useEffect(() => {
    viewRef.current?.dispatch({ effects: vimCompartment.current.reconfigure(vimMode ? vim() : []) });
  }, [vimMode]);

  // Blame in place, for the same reason. In an edit buffer the CodeMirror line
  // number IS the file line, so the lookup is direct.
  useEffect(() => {
    viewRef.current?.dispatch({
      effects: blameCompartment.current.reconfigure(blameExtension(props.blame)),
    });
  }, [props.blame]);

  // Expose save() to the parent.
  useEffect(() => {
    props.registerSave?.(() => { saveRef.current(); });
  }, [props.registerSave]);

  // Load file content whenever the file changes; rebuild state with its language.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    let cancelled = false;
    loadingRef.current = true;
    invoke<string>('read_file', { worktreePath: props.worktreePath, filePath: props.filePath })
      .then((content) => {
        if (cancelled || !viewRef.current) return;
        const lang = cmLangFor(props.languageId);
        const vimExt = vimCompartment.current.of(useStore.getState().vimMode ? vim() : []);
        // Both compartments have to be re-declared here — setState replaces the
        // configuration, so anything missing can never be reconfigured back in.
        const blameExt = blameCompartment.current.of(blameExtension(propsRef.current.blame));
        const base = [vimExt, ...baseExtensions, blameExt];
        const state = EditorState.create({
          doc: content,
          extensions: lang ? [...base, lang] : base,
        });
        view.setState(state);
        // Restore cursor + scroll, and re-push dynamic state (setState reset the field).
        const lineNo = Math.min(Math.max(1, props.initialCursorLine || 1), state.doc.lines);
        const lineObj = state.doc.line(lineNo);
        const pos = Math.min(lineObj.from + Math.max(0, (props.initialCursorCol || 1) - 1), lineObj.to);
        const effects: StateEffect<any>[] = [
          setEditorDyn.of({ ...dynSetsRef.current, fileAnnotations: propsRef.current.annotations, selStart: null, selEnd: null, portalEl: null, annContainers: annContainersRef.current }),
          setGrepQuery.of(useStore.getState().grepHighlight),
        ];
        if (props.initialCursorLine > 0) effects.push(EditorView.scrollIntoView(pos, { y: 'center' }));
        view.dispatch({ selection: { anchor: pos }, effects });
        loadingRef.current = false;
        propsRef.current.onModifiedChange(false);
      })
      .catch((e) => {
        if (cancelled || !viewRef.current) return;
        view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: `// Error loading file: ${e}` } });
        loadingRef.current = false;
      });
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.worktreePath, props.filePath, baseExtensions]);

  // Focus on a real open/commit. Preview opens reuse this editor instance and
  // never bump focusSignal, so this only fires for genuine opens; the isPreview
  // guard covers the fresh-mount case (e.g. a preview that swaps diff→edit view).
  useEffect(() => {
    if (props.focusSignal === undefined || props.isPreview) return;
    viewRef.current?.focus();
  }, [props.focusSignal, props.isPreview]);

  // Reflect the active content-search query into the editor's match highlight.
  useEffect(() => {
    viewRef.current?.dispatch({ effects: setGrepQuery.of(grepHighlight) });
  }, [grepHighlight]);

  // Push dynamic state into CM whenever annotations/threads or the selection change.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: setEditorDyn.of({
        ...dynSets,
        fileAnnotations: props.annotations,
        selStart: thisFileSel?.startLine ?? null,
        selEnd: thisFileSel?.endLine ?? null,
        portalEl: portalContainerRef.current,
        annContainers: annContainersRef.current,
      }),
    });
  }, [dynSets, props.annotations, thisFileSel?.startLine, thisFileSel?.endLine, portalContainer, annGroups]);

  return (
    <div
      className="code-editor-host"
      onMouseMove={(e) => {
        if (!ann.dragRange) return;
        const view = viewRef.current;
        if (!view) return;
        const pos = view.posAtCoords({ x: e.clientX, y: e.clientY }, false);
        if (pos === null) return;
        ann.extendDrag(repoId, filePath, view.state.doc.lineAt(pos).number);
      }}
    >
      <div ref={containerRef} className="code-editor" />
      <AnnotationPortals
        groups={annGroups}
        containers={annContainersRef.current}
        formEl={portalContainer}
        sel={thisFileSel}
        annotations={props.annotations}
        threads={props.threads}
        mr={props.mr}
        ann={ann}
        repoId={repoId}
        filePath={filePath}
      />
    </div>
  );
}
