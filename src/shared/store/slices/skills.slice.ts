import { invoke } from '../../ipc/invoke';
import type { StateCreator } from 'zustand';
import type { AgentSkill } from '../../ipc/ipc';
import type { AppState, SkillsSlice } from '../types';

export const skillsSlice: StateCreator<AppState, [], [], SkillsSlice> = (set) => ({
  skills: [],
  loadSkills: async () => {
    // A failure here costs the buttons, never the app: the skills still exist in
    // the agent, reachable by typing the slash command.
    try {
      set({ skills: await invoke<AgentSkill[]>('list_agent_skills') });
    } catch (e) {
      set({ lastError: `Failed to load skills: ${String(e)}` });
    }
  },
});
