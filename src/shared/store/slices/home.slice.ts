import { invoke } from '@tauri-apps/api/core';
import type { StateCreator } from 'zustand';
import type { HomeEntry, ReviewMr } from '../../ipc/ipc';
import type { AppState, HomeSlice } from '../types';
import { sessionTitle } from '../session';

export const homeSlice: StateCreator<AppState, [], [], HomeSlice> = (set, get) => ({
  // Review queue
  reviewQueue: null,
  refreshReviewQueue: async () => {
    try {
      const queue = await invoke<ReviewMr[]>('list_review_mrs');
      set({ reviewQueue: queue });
    } catch (e) {
      // Used to be a console warning only, which meant an empty review list was
      // indistinguishable from a broken `glab`.
      get().notify({
        kind: 'error',
        source: 'mr',
        title: 'Could not load the review queue',
        detail: String(e),
      });
    }
  },

  // Home snapshot
  homeSnapshot: null,
  homeLoading: false,
  refreshHome: async (forceMr = false) => {
    set({ homeLoading: true });
    try {
      set({ homeSnapshot: await invoke<HomeEntry[]>('get_home_snapshot', { forceMr }) });
    } catch (e) {
      get().notify({
        kind: 'error',
        source: 'app',
        title: 'Could not refresh Home',
        detail: String(e),
      });
    } finally {
      set({ homeLoading: false });
    }
  },

  // Task list
  tasks: [],
  setTasks: (tasks) => set({ tasks }),
  upsertTask: (task) =>
    set((s) => {
      const tasks = s.tasks.some((t) => t.short_id === task.short_id)
        ? s.tasks.map((t) => (t.short_id === task.short_id ? task : t))
        : [...s.tasks, task];
      // Keep any open session showing this task in sync (task object + tab label).
      let sessions = s.sessions;
      for (const id of s.sessionOrder) {
        const sess = s.sessions[id];
        if (sess?.task?.short_id === task.short_id) {
          if (sessions === s.sessions) sessions = { ...s.sessions };
          sessions[id] = { ...sess, task, title: sessionTitle(sess.kind, task) };
        }
      }
      return { tasks, sessions };
    }),
});
