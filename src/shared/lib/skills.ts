// What the agent is sent for a skill, and which skills a session offers. Pure.

import type { AgentSkill, SessionKind } from '../ipc/ipc';

/** The text typed for a skill. Keep the trailing space: it closes Claude's slash menu. */
export function skillCommand(skillId: string, args?: string): string {
  return `/${skillId} ${args ?? ''}`;
}

/** Strips trailing newlines and tabs; a trailing space must survive (see skillCommand). */
export function trimForPty(text: string): string {
  return text.replace(/[^\S ]+$/, '');
}

/** The skills a session's UI offers. Empty `kinds` means every kind. */
export function offeredSkills(skills: AgentSkill[], kind: SessionKind): AgentSkill[] {
  return skills.filter((s) => s.kinds.length === 0 || s.kinds.includes(kind));
}

/** Whether one skill is offered here. */
export function offers(skills: AgentSkill[], kind: SessionKind, id: string): boolean {
  return offeredSkills(skills, kind).some((s) => s.id === id);
}
