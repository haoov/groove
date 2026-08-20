import type { StateCreator } from 'zustand';
import type { ConfigView } from '../ipc/generated';
import type { Store } from './types';

/** Which top-level surface is showing. Session modes (overview/review) share the
 *  session chrome; home is the dashboard. */
export type View = 'home' | 'session' | 'overview' | 'review';

export interface UiSlice {
  view: View;
  setView: (v: View) => void;
  /** `undefined` while the first `get_config` is in flight; `null` = never set up. */
  config: ConfigView | null | undefined;
  setConfig: (c: ConfigView | null) => void;
  activeSessionId: string | null;
  setActiveSession: (id: string | null) => void;
}

export const uiSlice: StateCreator<Store, [], [], UiSlice> = (set) => ({
  view: 'home',
  setView: (view) => set({ view }),
  config: undefined,
  setConfig: (config) => set({ config }),
  activeSessionId: null,
  setActiveSession: (activeSessionId) => set({ activeSessionId }),
});
