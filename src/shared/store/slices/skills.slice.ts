import { invoke } from '../../ipc/invoke';
import type { StateCreator } from 'zustand';
import type { AgentSkill } from '../../ipc/ipc';
import type { AppState, SkillsSlice } from '../types';
import { errorText } from '../../lib/appError';

export const skillsSlice: StateCreator<AppState, [], [], SkillsSlice> = (set, get) => ({
  skills: [],
  skillsStale: false,
  setSkillsStale: (skillsStale) => set({ skillsStale }),
  loadSkills: async () => {
    try {
      set({ skills: await invoke<AgentSkill[]>('list_agent_skills') });
    } catch (e) {
      get().notify({ kind: 'error', title: 'Could not load the skills', detail: errorText(e) });
    }
  },
});
