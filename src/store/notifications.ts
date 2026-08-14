import { invoke } from '@tauri-apps/api/core';
import type { StoreApi } from 'zustand';
import type { AppNotification, AppState } from './types';

// The notification feed: one place decides what collapses, what is kept, and what
// escalates to the desktop. Everything that reports to the user goes through
// `notify`, so those three rules live here rather than at 40 call sites.

const NOTIFICATION_CAP = 200;
/** Identical events inside this window collapse into one row with a count. */
const NOTIFICATION_DEDUPE_MS = 10_000;
let notificationSeq = 0;

type NotificationState = Pick<
  AppState,
  | 'notifications'
  | 'toastIds'
  | 'notify'
  | 'dismissToast'
  | 'notificationsOpen'
  | 'setNotificationsOpen'
  | 'markNotificationRead'
  | 'markNotificationsRead'
  | 'clearNotifications'
>;

export function notificationSlice(set: StoreApi<AppState>['setState']): NotificationState {
  return {
  notifications: [],
  toastIds: [],
  notify: (input) =>
    set((s) => {
      const at = Date.now();
      // Collapse a repeat of the same event: bump its count and re-surface it,
      // rather than stacking twenty identical rows.
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
        // A success is an acknowledgement, not a record: it says "that worked",
        // which is worth a glance and nothing more. It lives exactly as long as
        // its toast (see dismissToast) and never reaches the feed or the badge.
        ephemeral: input.kind === 'success',
      };
      // A desktop notification is for when the app is NOT what you are looking
      // at; while it has focus the toast already said it. Scoped to the two kinds
      // that actually need a person — an agent that cannot continue, and a
      // failure — so the desktop never becomes a feed.
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
      // An ephemeral entry exists only to be shown once, so drop the record with
      // the toast rather than letting successes pile up unseen.
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
  };
}
