import { EditorView, drawSelection, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
import { search, highlightSelectionMatches } from '@codemirror/search';
import type { Extension } from '@codemirror/state';

/**
 * The view behaviour both editors share.
 *
 * Keymaps deliberately stay with each editor: the diff view is read-only and takes
 * only the search bindings, while the buffer view adds save, indent and history.
 * None of these are gutters, so where this sits in an extension array does not
 * change the gutter order.
 */
export function viewBasics(): Extension[] {
  return [
    highlightActiveLine(),
    highlightActiveLineGutter(),
    // Long lines wrap at the pane width; `.cm-content`'s 80ch min-width keeps them
    // from ever wrapping narrower than 80 columns.
    EditorView.lineWrapping,
    drawSelection(),
    highlightSelectionMatches(),
    search({ top: true }),
  ];
}
