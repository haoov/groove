import { invoke } from '../../ipc/invoke';
import type { StateCreator } from 'zustand';
import type { AppNotification, AppState, NotificationsSlice } from '../types';

// The notification feed: dedupe, cap and desktop escalation live here.

const NOTIFICATION_CAP = 200;
/** Identical events inside this window collapse into one row with a count. */
const NOTIFICATION_DEDUPE_MS = 10_000;
let notificationSeq = 0;

export const notificationsSlice: StateCreator<AppState, [], [], NotificationsSlice> = (set) => ({
  notifications: [],
  toastIds: [],
  notify: (input) =>
    set((s) => {
      const at = Date.now();
      // Collapse a repeat of the same event.
      const dupe = s.notifications.find(
        (n) =>
          n.title === input.title &&
          n.kind === input.kind &&
          n.taskId === input.taskId &&
          at - n.at < NOTIFICATION_DEDUPE_MS,
      );
      if (dupe) {
        return {
          notifications: s.notifications.map((n) =>
            n.id === dupe.id ? { ...n, count: n.count + 1, at, read: false } : n,
          ),
          toastIds: s.toastIds.includes(dupe.id) ? s.toastIds : [...s.toastIds, dupe.id],
        };
      }
      const entry: AppNotification = {
        source: 'app',
        ...input,
        id: `n-${++notificationSeq}`,
        at,
        read: false,
        count: 1,
        // A success lives as long as its toast; never in the feed or the badge.
        ephemeral: input.kind === 'success',
      };
      // Desktop notification only while the app lacks focus, for attention and error.
      if ((entry.kind === 'attention' || entry.kind === 'error') && !document.hasFocus()) {
        invoke('notify_desktop', {
          title: entry.title,
          body: entry.detail ?? '',
          urgency: entry.kind === 'error' ? 'critical' : 'normal',
        }).catch(console.warn);
      }
      return {
        notifications: [entry, ...s.notifications].slice(0, NOTIFICATION_CAP),
        toastIds: [...s.toastIds, entry.id],
      };
    }),
  dismissToast: (id) =>
    set((s) => ({
      toastIds: s.toastIds.filter((t) => t !== id),
      // An ephemeral entry goes with its toast.
      notifications: s.notifications.filter((n) => n.id !== id || !n.ephemeral),
    })),
  notificationsOpen: false,
  setNotificationsOpen: (v) => set({ notificationsOpen: v }),
  markNotificationRead: (id) =>
    set((s) => ({
      notifications: s.notifications.map((n) => (n.id === id ? { ...n, read: true } : n)),
    })),
  markNotificationsRead: () =>
    set((s) => ({ notifications: s.notifications.map((n) => ({ ...n, read: true })) })),
  clearNotifications: () => set({ notifications: [], toastIds: [] }),
});
