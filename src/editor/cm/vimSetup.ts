import { openSearchPanel, findNext, findPrevious } from '@codemirror/search';
import { Vim } from '@replit/codemirror-vim';

let done = false;

/**
 * Maps vim `/`, `?`, `n` and `N` to CodeMirror's search panel.
 * Idempotent: `Vim.mapCommand` mutates the vim keymap shared by every editor.
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
