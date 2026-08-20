import { StateEffect, StateField, Facet, type Extension } from '@codemirror/state';
import { Decoration, type DecorationSet, EditorView, WidgetType, keymap } from '@codemirror/view';
import type { Annotation } from '../shared/ipc/generated';

/** Actions the inline widgets invoke; supplied once per editor. */
export interface AnnHandlers {
  resolve: (id: string) => void;
  remove: (id: string) => void;
  create: (startLine: number, endLine: number, content: string) => void;
}

export const setAnnotations = StateEffect.define<Annotation[]>();
const startCompose = StateEffect.define<{ startLine: number; endLine: number }>();
const cancelCompose = StateEffect.define();

const handlersFacet = Facet.define<AnnHandlers, AnnHandlers>({
  combine: (v) => v[0] ?? { resolve: () => {}, remove: () => {}, create: () => {} },
});

interface AnnData { anns: Annotation[]; compose: { startLine: number; endLine: number } | null }

const annData = StateField.define<AnnData>({
  create: () => ({ anns: [], compose: null }),
  update(value, tr) {
    let next = value;
    for (const e of tr.effects) {
      if (e.is(setAnnotations)) next = { ...next, anns: e.value };
      else if (e.is(startCompose)) next = { ...next, compose: e.value };
      else if (e.is(cancelCompose)) next = { ...next, compose: null };
    }
    return next;
  },
});

class AnnotationWidget extends WidgetType {
  constructor(private ann: Annotation) { super(); }
  eq(o: AnnotationWidget) { return o.ann.id === this.ann.id && o.ann.status === this.ann.status && o.ann.content === this.ann.content; }
  toDOM(view: EditorView) {
    const h = view.state.facet(handlersFacet);
    const wrap = document.createElement('div');
    wrap.className = `cm-ann${this.ann.status === 'resolved' ? ' resolved' : ''}`;
    const body = document.createElement('div');
    body.className = 'cm-ann-body';
    body.textContent = this.ann.content;
    const foot = document.createElement('div');
    foot.className = 'cm-ann-foot';
    const who = document.createElement('span');
    who.className = 'cm-ann-author';
    who.textContent = this.ann.author;
    foot.append(who);
    if (this.ann.status !== 'resolved') foot.append(btn('Resolve', () => h.resolve(this.ann.id)));
    foot.append(btn('Delete', () => h.remove(this.ann.id)));
    wrap.append(body, foot);
    return wrap;
  }
  ignoreEvent() { return true; }
}

class ComposeWidget extends WidgetType {
  constructor(private startLine: number, private endLine: number) { super(); }
  toDOM(view: EditorView) {
    const h = view.state.facet(handlersFacet);
    const wrap = document.createElement('div');
    wrap.className = 'cm-ann compose';
    const ta = document.createElement('textarea');
    ta.className = 'cm-ann-input';
    ta.placeholder = 'Add a note…';
    ta.rows = 2;
    const foot = document.createElement('div');
    foot.className = 'cm-ann-foot';
    foot.append(
      btn('Cancel', () => view.dispatch({ effects: cancelCompose.of(null) })),
      btn('Save', () => {
        const text = ta.value.trim();
        if (text) h.create(this.startLine, this.endLine, text);
        view.dispatch({ effects: cancelCompose.of(null) });
      }),
    );
    wrap.append(ta, foot);
    setTimeout(() => ta.focus(), 0);
    return wrap;
  }
  ignoreEvent() { return true; }
}

function btn(label: string, onClick: () => void) {
  const b = document.createElement('button');
  b.textContent = label;
  b.onmousedown = (e) => { e.preventDefault(); onClick(); };
  return b;
}

const annDeco = EditorView.decorations.compute([annData], (state) => {
  const { anns, compose } = state.field(annData);
  const ranges = [];
  for (const a of anns) {
    const line = Math.min(Math.max(1, a.end_line), state.doc.lines);
    const pos = state.doc.line(line).to;
    ranges.push(Decoration.widget({ widget: new AnnotationWidget(a), block: true, side: 1 }).range(pos));
  }
  if (compose) {
    const line = Math.min(Math.max(1, compose.endLine), state.doc.lines);
    const pos = state.doc.line(line).to;
    ranges.push(Decoration.widget({ widget: new ComposeWidget(compose.startLine, compose.endLine), block: true, side: 1 }).range(pos));
  }
  ranges.sort((a, b) => a.from - b.from);
  return Decoration.set(ranges as never, true) as DecorationSet;
});

/** Open the compose widget for the current selection's line range. */
function composeForSelection(view: EditorView): boolean {
  const sel = view.state.selection.main;
  const startLine = view.state.doc.lineAt(sel.from).number;
  const endLine = view.state.doc.lineAt(sel.to).number;
  view.dispatch({ effects: startCompose.of({ startLine, endLine }) });
  return true;
}

/** Inline annotation display + compose, wired to the given handlers. */
export function annotationExtension(handlers: AnnHandlers): Extension {
  return [
    annData,
    annDeco,
    handlersFacet.of(handlers),
    keymap.of([{ key: 'Mod-Shift-a', preventDefault: true, run: composeForSelection }]),
  ];
}
