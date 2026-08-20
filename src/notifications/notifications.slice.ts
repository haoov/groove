import type { StateCreator } from 'zustand';
import type { Store } from '../shared/store/types';

export type ToastKind = 'info' | 'success' | 'error' | 'attention';

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  /** Optional click target — e.g. focus the agent that needs the user. */
  sessionId?: string;
}

let nextId = 1;

export interface NotificationsSlice {
  toasts: Toast[];
  notify: (kind: ToastKind, message: string, sessionId?: string) => number;
  dismissToast: (id: number) => void;
}

export const notificationsSlice: StateCreator<Store, [], [], NotificationsSlice> = (set) => ({
  toasts: [],

  notify: (kind, message, sessionId) => {
    const id = nextId++;
    set((st) => ({ toasts: [...st.toasts, { id, kind, message, sessionId }] }));
    return id;
  },

  dismissToast: (id) => set((st) => ({ toasts: st.toasts.filter((t) => t.id !== id) })),
});
