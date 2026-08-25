import type { StateCreator } from 'zustand';
import type { AppState, UiSlice } from '../types';

export const uiSlice: StateCreator<AppState, [], [], UiSlice> = (set) => ({
  // Navigation
  view: 'home',
  setView: (v) =>
    // Leaving the workspace drops a maximized agent: the rule that gives it the
    // pane area has no pane area to take on Home.
    set(v === 'workspace' ? { view: v } : { view: v, agentMaximized: false }),

  // Sidebar list focus
  panelFocusNonce: 0,
  requestPanelFocus: () => set((s) => ({ panelFocusNonce: s.panelFocusNonce + 1 })),
  commitFocusNonce: 0,
  requestCommitFocus: () =>
    set((s) => {
      const id = s.activeSessionId;
      const sess = id ? s.sessions[id] : null;
      return {
        commitFocusNonce: s.commitFocusNonce + 1,
        // Also un-collapse: the commit box cannot take focus while the column it
        // lives in is hidden.
        ...(sess && id
          ? {
              sessions: {
                ...s.sessions,
                [id]: { ...sess, sidebarTab: 'git' as const, sidebarCollapsed: false },
              },
            }
          : {}),
      };
    }),
  fileSearchFocusNonce: 0,
  fileSearchMode: 'name',
  requestFileSearchFocus: (mode = 'name') =>
    set((s) => ({ fileSearchMode: mode, fileSearchFocusNonce: s.fileSearchFocusNonce + 1 })),

  // Grep match highlight
  grepHighlight: null,
  // Under two characters is not a search worth painting — it would mark half the file.
  setGrepHighlight: (h) => set({ grepHighlight: h && h.query.length >= 2 ? h : null }),

  // Terminal focus
  terminalFocusReq: null,
  requestTerminalFocus: () => set((s) => ({ terminalFocusReq: (s.terminalFocusReq ?? 0) + 1 })),
  terminalConsoleOpen: false,
  setTerminalConsoleOpen: (v) => set({ terminalConsoleOpen: v }),

  // Command palette / overlays
  commandPaletteOpen: false,
  setCommandPaletteOpen: (v) => set({ commandPaletteOpen: v }),
  revealDir: null,
  revealInTree: (path) =>
    set((s) => ({ revealDir: { path, nonce: (s.revealDir?.nonce ?? 0) + 1 } })),
  openPicker: null,
  setOpenPicker: (p) => set({ openPicker: p }),
  pickerCursor: 0,
  setPickerCursor: (n) => set({ pickerCursor: n }),
  addRepoOpen: false,
  setAddRepoOpen: (v) => set({ addRepoOpen: v }),
  addWorktreeOpen: false,
  setAddWorktreeOpen: (v) => set({ addWorktreeOpen: v }),

  settingsOpen: false,
  setSettingsOpen: (v) => set({ settingsOpen: v }),

  // Vim mode — defaults on (the editor + readonly-diff use vim navigation).
  vimMode: (() => {
    try {
      const v = localStorage.getItem('workbench.vimMode');
      return v === null ? true : v === 'true';
    } catch {
      return true;
    }
  })(),
  setVimMode: (v) => {
    try { localStorage.setItem('workbench.vimMode', String(v)); } catch { /* ignore */ }
    set({ vimMode: v });
  },

  // Task wizard
});
