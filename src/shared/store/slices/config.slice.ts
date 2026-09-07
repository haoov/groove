import { invoke } from '../../ipc/invoke';
import type { StateCreator } from 'zustand';
import { applyTheme, applyFontSize, applyFontFamily } from '../../lib/theme';
import type { AppState, ConfigSlice } from '../types';

/** A config write is fire-and-forget: a failure only shows in the feed. */
const fail = (get: () => AppState, title: string) => (e: unknown) =>
  get().notify({ kind: 'error', title, detail: String(e) });

export const configSlice: StateCreator<AppState, [], [], ConfigSlice> = (set, get) => ({
  config: null,
  setConfig: (c) => set({ config: c }),
  setTheme: (theme) => {
    applyTheme(theme);
    invoke('set_theme', { theme }).catch(fail(get, 'Could not save the theme'));
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, theme } } } : {}));
  },
  setFontSize: (px) => {
    applyFontSize(px);
    invoke('set_font_size', { fontSize: px }).catch(fail(get, 'Could not save the font size'));
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, font_size: px } } } : {}));
  },
  setSuggestActions: (v) => {
    invoke('set_suggest_actions', { suggestActions: v }).catch(fail(get, 'Could not save the action suggestions'));
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, suggest_actions: v } } } : {}));
  },
  setFontFamily: (family) => {
    applyFontFamily(family);
    invoke('set_font_family', { fontFamily: family }).catch(fail(get, 'Could not save the font'));
    // The store update triggers terminalHost's re-skin.
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, font_family: family } } } : {}));
  },
  setAgentFontFamily: (family) => {
    // No CSS token: only xterm reads it, through terminalHost's subscriber.
    invoke('set_agent_font_family', { agentFontFamily: family }).catch(fail(get, 'Could not save the agent font'));
    set((s) => (s.config ? { config: { ...s.config, ui: { ...s.config.ui, agent_font_family: family } } } : {}));
  },

  // Status
  syncStatus: 'idle',
  setSyncStatus: (s) => set({ syncStatus: s }),
  setLastError: (e) => {
    // Every error lands in the feed. Call sites that know more should `notify` directly.
    if (e) get().notify({ kind: 'error', title: e });
  },
});
