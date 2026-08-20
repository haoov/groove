import type { StateCreator } from 'zustand';
import { call } from '../shared/ipc/client';
import type { HomeEntry, TaskView, ReviewMr } from '../shared/ipc/generated';
import type { Store } from '../shared/store/types';

export interface HomeSlice {
  /** Per-session local state (repos, worktrees, MR signals). Null until first fetch. */
  homeSnapshot: HomeEntry[] | null;
  /** The Notion queue mirror. */
  tasks: TaskView[];
  /** Open MRs where the user is a reviewer. Null until first fetch (CLI may be absent). */
  reviewQueue: ReviewMr[] | null;
  homeLoading: boolean;
  /** `force` bypasses the cached CI/thread signals (manual refresh). */
  refreshHome: (force?: boolean) => Promise<void>;
  refreshTasks: () => Promise<void>;
  refreshQueue: () => Promise<void>;
}

export const homeSlice: StateCreator<Store, [], [], HomeSlice> = (set) => ({
  homeSnapshot: null,
  tasks: [],
  reviewQueue: null,
  homeLoading: false,

  refreshHome: async (force) => {
    set({ homeLoading: true });
    try {
      set({ homeSnapshot: await call<HomeEntry[]>('get_home_snapshot', { forceMr: !!force }) });
    } catch (e) {
      console.warn('get_home_snapshot failed', e);
    } finally {
      set({ homeLoading: false });
    }
  },

  refreshTasks: async () => {
    try {
      set({ tasks: await call<TaskView[]>('list_tasks') });
    } catch (e) {
      console.warn('list_tasks failed', e);
    }
  },

  refreshQueue: async () => {
    try {
      set({ reviewQueue: await call<ReviewMr[]>('list_review_mrs') });
    } catch (e) {
      console.warn('list_review_mrs failed', e);
    }
  },
});
