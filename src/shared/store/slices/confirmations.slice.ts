import type { StateCreator } from 'zustand';
import type { AppState, ConfirmationsSlice } from '../types';

export const confirmationsSlice: StateCreator<AppState, [], [], ConfirmationsSlice> = (set) => ({
  pendingConfirmations: [],
  addConfirmation: (c) =>
    set((s) => ({
      pendingConfirmations: s.pendingConfirmations.some((p) => p.id === c.id)
        ? s.pendingConfirmations
        : [...s.pendingConfirmations, c],
      // A new request always surfaces the modal, even if the user deferred earlier.
      confirmationsMinimized: false,
    })),
  removeConfirmation: (id) =>
    set((s) => ({
      pendingConfirmations: s.pendingConfirmations.filter((c) => c.id !== id),
    })),
  confirmationsMinimized: false,
  setConfirmationsMinimized: (v) => set({ confirmationsMinimized: v }),
});
