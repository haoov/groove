import { invoke } from '../../ipc/invoke';
import type { StateCreator } from 'zustand';
import type { AgentSkill } from '../../ipc/ipc';
import type { AppState, SkillsSlice } from '../types';

export const skillsSlice: StateCreator<AppState, [], [], SkillsSlice> = (set) => ({
  skills: [],
  skillsStale: false,
  setSkillsStale: (skillsStale) => set({ skillsStale }),
  loadSkills: async () => {
    // A failure costs the buttons, not the app.
    try {
      set({ skills: await invoke<AgentSkill[]>('list_agent_skills') });
    } catch (e) {
      set({ lastError: `Failed to load skills: ${String(e)}` });
    }
  },
});
