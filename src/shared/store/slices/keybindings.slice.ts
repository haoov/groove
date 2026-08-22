import type { StateCreator } from 'zustand';
import { assignBinding, defaultKeymap, loadKeymap, saveKeymap, clearKeymap } from '../../lib/keybindings';
import type { AppState, KeybindingsSlice } from '../types';

export const keybindingsSlice: StateCreator<AppState, [], [], KeybindingsSlice> = (set) => ({
  keymap: loadKeymap(),
  setBinding: (id, chords) =>
    set((s) => {
      // Exclusive: the chord is taken off whatever held it. Otherwise two
      // commands share it and declaration order silently decides the winner.
      const keymap = assignBinding(s.keymap, id, chords);
      saveKeymap(keymap);
      return { keymap };
    }),
  resetKeymap: () => {
    clearKeymap();
    set({ keymap: defaultKeymap() });
  },
  capturingKey: false,
  setCapturingKey: (v) => set({ capturingKey: v }),
});
