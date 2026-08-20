// The app store: feature slices composed into one. Add a slice by importing its
// creator here and spreading it, and by extending Store in ./types.

import { create } from 'zustand';
import type { Store } from './types';
import { uiSlice } from './ui.slice';
import { homeSlice } from '../../home/home.slice';
import { sessionsSlice } from '../../sessions/sessions.slice';

export type { View } from './ui.slice';
export type { Store } from './types';

export const useStore = create<Store>()((...a) => ({
  ...uiSlice(...a),
  ...homeSlice(...a),
  ...sessionsSlice(...a),
}));
