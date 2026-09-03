// What the agent is sent for a skill, and which skills a session offers.
//
// Pure, and separate from `agentSend` on purpose: both halves are details a
// reader would tidy away, so both are testable without a PTY.

import type { AgentSkill, SessionKind } from '../ipc/ipc';

/**
 * The literal text typed into the agent for a skill.
 *
 * The TRAILING SPACE is load-bearing. Typing `/` opens Claude's slash menu, and
 * with the menu open Enter acts on the highlighted row — which is not always the
 * skill named, since the menu ranks by prefix and past use. A space closes it.
 */
export function skillCommand(skillId: string, args?: string): string {
  return `/${skillId} ${args ?? ''}`;
}

/**
 * Trailing whitespace to strip before writing to the PTY: newlines and tabs
 * become blank lines in the draft, a SPACE must survive (see `skillCommand`).
 */
export function trimForPty(text: string): string {
  return text.replace(/[^\S ]+$/, '');
}

/** The skills a session's UI offers. Empty `kinds` means every kind. */
export function offeredSkills(skills: AgentSkill[], kind: SessionKind): AgentSkill[] {
  return skills.filter((s) => s.kinds.length === 0 || s.kinds.includes(kind));
}

/** Whether one skill is offered here — for a surface that names its own. */
export function offers(skills: AgentSkill[], kind: SessionKind, id: string): boolean {
  return offeredSkills(skills, kind).some((s) => s.id === id);
}
