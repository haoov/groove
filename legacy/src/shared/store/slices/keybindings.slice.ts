import type { StateCreator } from 'zustand';
import { assignBinding, defaultKeymap, loadKeymap, saveKeymap, clearKeymap, resetBinding } from '../../lib/pure/keybindings';
import type { AppState, KeybindingsSlice } from '../types';

export const keybindingsSlice: StateCreator<AppState, [], [], KeybindingsSlice> = (set) => ({
  keymap: loadKeymap(),
  setBinding: (id, chords) =>
    set((s) => {
      // Exclusive: the chord comes off whatever held it.
      const keymap = assignBinding(s.keymap, id, chords);
      saveKeymap(keymap);
      return { keymap };
    }),
  resetBinding: (id) =>
    set((s) => {
      const keymap = resetBinding(s.keymap, id);
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
