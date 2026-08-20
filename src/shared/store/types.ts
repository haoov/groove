// The composed store type. Each feature contributes a slice; this is the one
// place `shared` references features, and it's type-only (no runtime cycle).

import type { UiSlice } from './ui.slice';
import type { HomeSlice } from '../../home/home.slice';
import type { SessionsSlice } from '../../sessions/sessions.slice';
import type { ApprovalsSlice } from '../../approvals/approvals.slice';
import type { NotificationsSlice } from '../../notifications/notifications.slice';

export type Store = UiSlice & HomeSlice & SessionsSlice & ApprovalsSlice & NotificationsSlice;
