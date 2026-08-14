import { describe, expect, it } from 'vitest';
import {
  closePaneReducer, closeTabReducer, commitPreviewReducer, discardPreviewReducer,
  isTerminalPane, newWorkspaceSession, openTabReducer, sessionTitle, splitPaneReducer,
} from './session';
import type { SessionState } from './types';
import type { Task } from '../types/ipc';

// The reducers behind every pane and tab interaction. Each bug fixed here has been
// a real one a user hit: a second terminal focusing the first, a preview tab
// demoting a real one, a split collapsing and losing a terminal.

const task = (short = 'TASKS2-1'): Task => ({
  short_id: short, page_id: 'p', title: 'Some task', status: 'In progress',
  priority: 'P2', task_type: 'Bug', url: '', repos: [],
} as unknown as Task);

const fresh = (): SessionState => newWorkspaceSession('task', task(), [], []);
const pane = (s: Partial<SessionState>, i = 0) => (s.panes ?? [])[i];
const tabIds = (s: Partial<SessionState>, i = 0) => (pane(s, i)?.tabs ?? []).map((t) => t.id);
const apply = (s: SessionState, patch: Partial<SessionState>): SessionState => ({ ...s, ...patch });

describe('newWorkspaceSession', () => {
  it('starts with one pane holding the overview', () => {
    const s = fresh();
    expect(s.panes).toHaveLength(1);
    expect(tabIds(s)).toEqual(['::overview']);
    expect(s.activePaneId).toBe(s.panes[0].id);
  });

  it('gives each session its own containers', () => {
    const a = fresh();
    const b = fresh();
    expect(a.expandedDiffFiles).not.toBe(b.expandedDiffFiles);
    expect(a.diffHunks).not.toBe(b.diffHunks);
    expect(a.panes[0].tabs).not.toBe(b.panes[0].tabs);
  });
});

describe('sessionTitle', () => {
  it('names a task session after its task', () => {
    expect(sessionTitle('task', task('TASKS2-42'))).toContain('TASKS2-42');
  });

  it('has a title with no task', () => {
    expect(sessionTitle('explorer', null)).toBeTruthy();
  });
});

describe('openTabReducer', () => {
  it('opens a file tab and focuses it', () => {
    const s = fresh();
    const next = openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' });
    expect(tabIds(next)).toEqual(['::overview', 'r1::a.ts']);
    expect(pane(next).activeTabId).toBe('r1::a.ts');
  });

  it('reuses the tab for a file already open, switching its view', () => {
    let s = apply(fresh(), openTabReducer(fresh(), { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'diff' }));
    expect(tabIds(s)).toEqual(['::overview', 'r1::a.ts']);
    expect(pane(s).tabs[1].view).toBe('diff');
  });

  it('keys a commit tab on its sha and an MR tab on its id', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: '', view: 'diff', kind: 'commit', sha: 'abc' }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: '', view: 'diff', kind: 'commit', sha: 'def' }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: '', view: 'diff', kind: 'mr', mrId: '7' }));
    expect(tabIds(s)).toEqual(['::overview', 'r1::commit::abc', 'r1::commit::def', 'r1::mr::7']);
  });

  // The bug: the tab id fell back to the label, and every unbound terminal is
  // labelled 'Terminal', so they collapsed into one tab.
  it('gives each unbound terminal its own tab', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal', label: 'Terminal' }));
    s = apply(s, openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal', label: 'Terminal' }));
    s = apply(s, openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal', label: 'Terminal' }));
    const terms = s.panes.flatMap((p) => p.tabs).filter((t) => t.kind === 'terminal');
    expect(new Set(terms.map((t) => t.id)).size).toBe(3);
  });

  it('treats a terminal bound to the same PTY session as the same tab', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal', ptySessionId: 'pty-1' }));
    s = apply(s, openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal', ptySessionId: 'pty-1' }));
    expect(s.panes.flatMap((p) => p.tabs).filter((t) => t.kind === 'terminal')).toHaveLength(1);
  });

  it('focuses a single-instance tab where it already lives instead of copying it', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'row'));
    const other = s.panes[1].id;
    // The overview lives in pane 1; asking for it from pane 2 must not duplicate it.
    const next = openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'overview' }, { paneId: other });
    expect(next.activePaneId).toBe(s.panes[0].id);
    const all = (next.panes ?? []).flatMap((p) => p.tabs).filter((t) => t.kind === 'overview');
    expect(all).toHaveLength(1);
  });

  it('never opens a single-instance kind as a preview', () => {
    const s = fresh();
    const next = openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal', preview: true });
    const term = (next.panes ?? []).flatMap((p) => p.tabs).find((t) => t.kind === 'terminal');
    expect(term?.preview).toBeFalsy();
  });

  it('reuses the single preview slot rather than stacking previews', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit', preview: true }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'b.ts', view: 'edit', preview: true }));
    expect(tabIds(s)).toEqual(['::overview', 'r1::b.ts']);
  });

  it('promotes a preview when the same file is opened for real', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit', preview: true }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    expect(pane(s).tabs[1].preview).toBe(false);
  });

  // A tab opened normally carries no `preview` key at all, so the flag reads
  // undefined rather than false. Only its falsiness is load-bearing.
  it('never demotes a real tab to a preview', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit', preview: true }));
    expect(pane(s).tabs[1].preview).toBeFalsy();
  });
});

describe('commitPreviewReducer / discardPreviewReducer', () => {
  it('keeps the previewed tab on commit', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit', preview: true }));
    s = apply(s, commitPreviewReducer(s, s.activePaneId));
    expect(tabIds(s)).toEqual(['::overview', 'r1::a.ts']);
    expect(pane(s).tabs[1].preview).toBe(false);
  });

  it('removes it on discard and falls back to the tab before it', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit', preview: true }));
    s = apply(s, discardPreviewReducer(s, s.activePaneId));
    expect(tabIds(s)).toEqual(['::overview']);
    expect(pane(s).activeTabId).toBe('::overview');
  });

  it('never collapses a split pane the user made, even when left empty', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'row'));
    const newPane = s.panes[1].id;
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit', preview: true }, { paneId: newPane }));
    s = apply(s, discardPreviewReducer(s, newPane));
    expect(s.panes).toHaveLength(2);
  });
});

describe('closeTabReducer', () => {
  it('activates the tab to the left', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'b.ts', view: 'edit' }));
    s = apply(s, closeTabReducer(s, s.activePaneId, 'r1::b.ts'));
    expect(pane(s).activeTabId).toBe('r1::a.ts');
  });

  it('leaves the active tab alone when closing another one', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'b.ts', view: 'edit' }));
    s = apply(s, closeTabReducer(s, s.activePaneId, 'r1::a.ts'));
    expect(pane(s).activeTabId).toBe('r1::b.ts');
  });

  it('closes the pane when its last tab goes', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    s = apply(s, splitPaneReducer(s, 'row'));          // pane 2 clones a.ts
    const second = s.panes[1].id;
    s = apply(s, closeTabReducer(s, second, 'r1::a.ts'));
    expect(s.panes).toHaveLength(1);
  });

  it('keeps the only pane, empty, rather than leaving no pane at all', () => {
    const solo = fresh();
    const after = closeTabReducer(solo, solo.activePaneId, '::overview');
    expect((after.panes ?? []).length).toBe(1);
    expect(tabIds(after)).toEqual([]);
  });

  // A split whose source tab was single-instance produces a pane with no tabs.
  // That pane is meant to survive — it is where "open a terminal here" lands.
  it('leaves an empty split pane alone', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'row'));
    expect(s.panes[1].tabs).toEqual([]);
    expect(s.panes).toHaveLength(2);
  });
});

describe('splitPaneReducer', () => {
  it('clones the active tab into the new pane and focuses it', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    const next = splitPaneReducer(s, 'row');
    expect(next.panes).toHaveLength(2);
    expect(next.activePaneId).toBe(next.panes![1].id);
    expect(next.panes![1].tabs.map((t) => t.id)).toEqual(['r1::a.ts']);
  });

  it('leaves the new pane empty rather than cloning a single-instance tab', () => {
    const s = fresh(); // active tab is the overview
    const next = splitPaneReducer(s, 'row');
    expect(next.panes![1].tabs).toEqual([]);
    expect(next.panes![1].activeTabId).toBeNull();
  });

  it('clones without carrying the preview flag', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit', preview: true }));
    const next = splitPaneReducer(s, 'row');
    expect(next.panes![1].tabs[0].preview).toBe(false);
  });
});

describe('the terminal dock', () => {
  const term = (s: SessionState, paneId?: string) =>
    openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal' }, paneId ? { paneId } : undefined);

  it('is a pane that holds only terminals', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'col'));      // empty pane
    s = apply(s, term(s));
    const dock = s.panes.find((p) => isTerminalPane(p));
    expect(dock).toBeDefined();
    expect(dock!.tabs.every((t) => t.kind === 'terminal')).toBe(true);
    // An empty pane is neither kind — it can still become either.
    expect(isTerminalPane({ id: 'x', tabs: [], activeTabId: null })).toBe(false);
  });

  it('keeps a file out of the dock, even when the dock is the active pane', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'col'));
    s = apply(s, term(s));
    const dockId = s.panes.find((p) => isTerminalPane(p))!.id;
    expect(s.activePaneId).toBe(dockId);

    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    const dock = s.panes.find((p) => p.id === dockId)!;
    expect(dock.tabs.every((t) => t.kind === 'terminal')).toBe(true);
    expect(s.panes.some((p) => p.tabs.some((t) => t.id === 'r1::a.ts'))).toBe(true);
  });

  it('keeps a terminal out of an editor pane, even when asked directly', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'col'));
    s = apply(s, term(s));
    const editorId = s.panes.find((p) => !isTerminalPane(p))!.id;
    s = apply(s, term(s, editorId));
    const editor = s.panes.find((p) => p.id === editorId)!;
    expect(editor.tabs.some((t) => t.kind === 'terminal')).toBe(false);
  });

  it('splits into another terminal, side by side, never an empty pane', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'col'));
    s = apply(s, term(s));
    const before = s.panes.length;
    s = apply(s, splitPaneReducer(s, 'col'));   // asks for a vertical split
    expect(s.panes).toHaveLength(before + 1);
    const added = s.panes[s.panes.length - 1];
    expect(added.tabs).toHaveLength(1);
    expect(added.tabs[0].kind).toBe('terminal');
    expect(added.activeTabId).toBe(added.tabs[0].id);
    expect(s.activePaneId).toBe(added.id);
  });

  it('gives each split terminal its own tab id, so they are separate PTYs', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'col'));
    s = apply(s, term(s));
    s = apply(s, splitPaneReducer(s, 'row'));
    s = apply(s, splitPaneReducer(s, 'row'));
    const ids = s.panes.flatMap((p) => p.tabs).filter((t) => t.kind === 'terminal').map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toHaveLength(3);
  });

  it('still clones a normal tab when splitting an editor pane', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    const next = splitPaneReducer(s, 'row');
    expect(next.panes![1].tabs[0].id).toBe('r1::a.ts');
  });
});

describe('closePaneReducer', () => {
  it('refuses to close the only pane', () => {
    const s = fresh();
    expect(closePaneReducer(s, s.activePaneId)).toEqual({});
  });

  it('merges the closed pane tabs into the survivor without duplicating', () => {
    let s = fresh();
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'a.ts', view: 'edit' }));
    s = apply(s, splitPaneReducer(s, 'row'));          // pane 2 clones a.ts
    const second = s.panes[1].id;
    s = apply(s, openTabReducer(s, { repoId: 'r1', filePath: 'b.ts', view: 'edit' }, { paneId: second }));
    s = apply(s, closePaneReducer(s, second));
    expect(s.panes).toHaveLength(1);
    expect(tabIds(s)).toEqual(['::overview', 'r1::a.ts', 'r1::b.ts']);
  });

  it('carries a terminal over instead of destroying it', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'row'));
    const second = s.panes[1].id;
    s = apply(s, openTabReducer(s, { repoId: '', filePath: '', view: 'diff', kind: 'terminal', ptySessionId: 'pty-9' }, { paneId: second }));
    s = apply(s, closePaneReducer(s, second));
    const terms = s.panes.flatMap((p) => p.tabs).filter((t) => t.ptySessionId === 'pty-9');
    expect(terms).toHaveLength(1);
  });

  it('moves focus and clears a maximize that pointed at the closed pane', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'row'));
    const second = s.panes[1].id;
    s = { ...s, maximizedPaneId: second, activePaneId: second };
    const next = closePaneReducer(s, second);
    expect(next.maximizedPaneId).toBeNull();
    expect(next.activePaneId).toBe(s.panes[0].id);
  });

  it('ignores an unknown pane id', () => {
    let s = fresh();
    s = apply(s, splitPaneReducer(s, 'row'));
    expect(closePaneReducer(s, 'pane-nope')).toEqual({});
  });
});
