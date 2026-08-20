import { openSearchPanel, findNext, findPrevious } from '@codemirror/search';
import { Vim } from '@replit/codemirror-vim';

let done = false;

/**
 * Replace vim's bare in-buffer `/` search with CodeMirror's search panel, which
 * supports regex, case toggles, and replace — while keeping every other vim
 * motion. `n`/`N` navigate the CM matches so the vim search-flow still works.
 *
 * Global + idempotent: `Vim.mapCommand` mutates the shared vim keymap, so one
 * call covers every vim editor (diff + edit). The action adapter exposes the
 * underlying EditorView as `cm.cm6`.
 */
export function setupVimSearch() {
  if (done) return;
  done = true;

  Vim.defineAction('openCmSearchPanel', (cm: { cm6: Parameters<typeof openSearchPanel>[0] }) => {
    openSearchPanel(cm.cm6);
  });
  Vim.defineAction('cmFindNext', (cm: { cm6: Parameters<typeof findNext>[0] }) => {
    findNext(cm.cm6);
  });
  Vim.defineAction('cmFindPrev', (cm: { cm6: Parameters<typeof findPrevious>[0] }) => {
    findPrevious(cm.cm6);
  });

  Vim.mapCommand('/', 'action', 'openCmSearchPanel', {}, { context: 'normal' });
  Vim.mapCommand('?', 'action', 'openCmSearchPanel', {}, { context: 'normal' });
  Vim.mapCommand('n', 'action', 'cmFindNext', {}, { context: 'normal' });
  Vim.mapCommand('N', 'action', 'cmFindPrev', {}, { context: 'normal' });
}
