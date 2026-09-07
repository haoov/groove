// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, waitFor, act } from '@testing-library/react';
import { useSession, type EditorTab } from '../shared/store';
import { resetStore, seedSession, testRepo, testWorktree } from '../shared/store/testing';
import type { Hunk } from '../shared/ipc/ipc';
import type { AnnCtx } from '../editor/useAnnotations';
import { WorkspacePane } from './WorkspacePane';

vi.mock('../shared/ipc/invoke', () => ({ invoke: vi.fn() }));
import { invoke } from '../shared/ipc/invoke';

// CodeMirror measures a real layout; the diff body is asserted through this stand-in.
vi.mock('../editor/FileDiffEditor', () => ({
  FileDiffEditor: ({ hunks }: { hunks: Hunk[] }) => (
    <div data-testid="diff-body">{hunks.flatMap((h) => h.lines.map((l) => l.content)).join('\n')}</div>
  ),
}));

const invoked = vi.mocked(invoke);

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => { resolve = r; });
  return { promise, resolve };
}

const hunksFor = (content: string): Hunk[] => [
  { header: '@@ -1 +1 @@', lines: [{ num: 1, content, type: 'add' }] },
];

const fileTab = (filePath: string): EditorTab => ({
  id: `repo-1::${filePath}`,
  repoId: 'repo-1',
  filePath,
  view: 'diff',
  kind: 'file',
});

const A = 'src/a.ts';
const B = 'src/b.ts';

const ann = {
  sel: null,
  dragRange: null,
  annotationText: '',
  replyTexts: {},
  replyPending: {},
  postPending: {},
  editingId: null,
  editText: '',
  editPending: {},
  resolvePending: {},
  deletePending: {},
  inputRef: { current: null },
  setAnnotationText: () => {},
  beginDrag: () => {},
  extendDrag: () => {},
  selectSingle: () => {},
  submit: () => {},
  cancel: () => {},
  setReplyTexts: () => {},
  submitReply: () => {},
  postToMr: () => {},
  setEditText: () => {},
  beginEdit: () => {},
  cancelEdit: () => {},
  saveEdit: () => {},
  resolveNote: () => {},
  deleteAnnotation: () => {},
  openInEditor: () => {},
} as unknown as AnnCtx;

/** Renders the session's only pane, re-reading it from the store like WorkspaceLayout does. */
function Pane() {
  const pane = useSession((s) => s.panes[0]);
  return <WorkspacePane pane={pane} ann={ann} isActive multiRepo={false} />;
}

/** The pending `get_file_diff` reply for one file. */
let replies: Record<string, ReturnType<typeof deferred<Hunk[]>>>;

const diffCallsFor = (filePath: string) =>
  invoked.mock.calls.filter(
    ([cmd, args]) => cmd === 'get_file_diff' && (args as { filePath: string }).filePath === filePath,
  );

beforeEach(() => {
  resetStore();
  replies = { [A]: deferred<Hunk[]>(), [B]: deferred<Hunk[]>() };
  invoked.mockReset();
  invoked.mockImplementation((cmd: string, args?: unknown) => {
    if (cmd === 'get_file_diff') return replies[(args as { filePath: string }).filePath].promise as never;
    if (cmd === 'read_file_lines') return Promise.resolve({ total: 1, lines: [] }) as never;
    return Promise.resolve(undefined) as never;
  });
  seedSession({
    repos: [testRepo()],
    worktrees: [testWorktree()],
    patch: {
      panes: [{ id: 'pane-1', tabs: [fileTab(A), fileTab(B)], activeTabId: `repo-1::${A}` }],
      activePaneId: 'pane-1',
      workspaceMode: 'code',
    },
  });
});

afterEach(cleanup);

const clickTab = (filePath: string) =>
  screen.getByTitle(`mayo · ${filePath}`).dispatchEvent(
    new MouseEvent('mousedown', { bubbles: true, button: 0 }),
  );

describe('the diff tab', () => {
  it('shows a loading line until the diff arrives, then the file content', async () => {
    render(<Pane />);
    expect(screen.getByText('Loading diff…')).toBeTruthy();

    await act(async () => { replies[A].resolve(hunksFor('the A body')); });
    expect(screen.getByTestId('diff-body').textContent).toBe('the A body');
  });

  it('requests and renders the second file after a tab switch', async () => {
    render(<Pane />);
    await act(async () => { replies[A].resolve(hunksFor('the A body')); });

    await act(async () => { clickTab(B); });
    expect(diffCallsFor(B)).toHaveLength(1);
    expect(screen.getByText('Loading diff…')).toBeTruthy();

    await act(async () => { replies[B].resolve(hunksFor('the B body')); });
    await waitFor(() => expect(screen.getByTestId('diff-body').textContent).toBe('the B body'));
  });

  it('never leaves the second tab loading when the first file is still in flight', async () => {
    render(<Pane />);
    await act(async () => { clickTab(B); });
    await act(async () => { replies[B].resolve(hunksFor('the B body')); });

    await waitFor(() => expect(screen.getByTestId('diff-body').textContent).toBe('the B body'));
  });

  it('keeps the second tab on its own diff when the first file answers late', async () => {
    render(<Pane />);
    await act(async () => { clickTab(B); });
    await act(async () => { replies[B].resolve(hunksFor('the B body')); });

    await act(async () => { replies[A].resolve(hunksFor('the A body')); });
    expect(screen.getByTestId('diff-body').textContent).toBe('the B body');
  });

  it('reuses the cached diff instead of refetching a file already fetched', async () => {
    render(<Pane />);
    await act(async () => { replies[A].resolve(hunksFor('the A body')); });
    await act(async () => { clickTab(B); });
    await act(async () => { replies[B].resolve(hunksFor('the B body')); });
    await act(async () => { clickTab(A); });

    expect(diffCallsFor(A)).toHaveLength(1);
    expect(screen.getByTestId('diff-body').textContent).toBe('the A body');
  });
});
