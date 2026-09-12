import { describe, expect, it } from 'vitest';
import { useStore, sessionActions } from './index';

/** `buildView` caches on the session object, so every `useSession` selector re-runs when
 *  its identity changes. A patch that changes nothing must keep it. */
describe('session identity', () => {
  const open = () => {
    const id = useStore.getState().openSession({ kind: 'explorer' });
    return { id, session: () => useStore.getState().sessions[id] };
  };

  it('survives a patch that sets a field to what it already holds', () => {
    const { id, session } = open();
    const before = session();
    sessionActions(id).setSidebarTab(before.sidebarTab);
    expect(session()).toBe(before);
  });

  it('survives a recipe that declines to change anything', () => {
    const { id, session } = open();
    sessionActions(id).setCommits([], false);
    const before = session();
    // No more pages: the recipe returns an empty patch.
    sessionActions(id).loadMoreCommits();
    expect(session()).toBe(before);
  });

  it('survives re-selecting the worktree already active', () => {
    const { id, session } = open();
    const before = session();
    sessionActions(id).setActiveWorktreeId(before.activeWorktreeId);
    expect(session()).toBe(before);
  });

  it('changes on a real edit', () => {
    const { id, session } = open();
    const before = session();
    sessionActions(id).setSidebarTab(before.sidebarTab === 'git' ? 'files' : 'git');
    expect(session()).not.toBe(before);
  });
});
