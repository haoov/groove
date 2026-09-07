import { EditorView, drawSelection, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
import { search, highlightSelectionMatches } from '@codemirror/search';
import type { Extension } from '@codemirror/state';

/** The view extensions both editors share. Keymaps stay with each editor. */
export function viewBasics(): Extension[] {
  return [
    highlightActiveLine(),
    highlightActiveLineGutter(),
    // The 80ch min-width on `.cm-content` in CSS sets the wrap floor.
    EditorView.lineWrapping,
    drawSelection(),
    highlightSelectionMatches(),
    search({ top: true }),
  ];
}
