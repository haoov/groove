// The contract between the main window and the detached agent window: events, mirrored
// state, commands, persisted layout. Pure: no Tauri import.

import type { AgentActivity, AgentSkill, Config, SessionKind } from '../ipc/ipc';
import { clampAgentsWidth, type AgentRow } from './agents';

export const AGENT_WINDOW_LABEL = 'agent';

/** Frontend-to-frontend events. Not in shared/ipc/events.ts: the backend never sees them. */
export const BRIDGE = {
  /** main → agent: the whole mirrored state, on every change and on READY. */
  STATE: 'agent-window:state',
  /** agent → main: the window has mounted and wants the state. */
  READY: 'agent-window:ready',
  /** agent → main: an action the main window runs on its behalf. */
  COMMAND: 'agent-window:command',
  /** main → agent: a command finished, with the error when it failed. */
  DONE: 'agent-window:done',
} as const;

/** Everything the detached window renders. The main window is the source of truth. */
export interface AgentWindowState {
  /** The focused session a session-scoped command is answered against. */
  sessionId: string | null;
  taskId: string | null;
  ptyId: string | null;
  kind: SessionKind;
  activity: AgentActivity | null;
  autoApprove: boolean;
  skills: AgentSkill[];
  skillsStale: boolean;
  /** The config view (no tokens): theme, fonts and which task sources exist. */
  config: Config | null;
  /** Every open session and its agent, for the running-agents list. */
  agents: AgentRow[];
  agentsOpen: boolean;
}

export type AgentWindowCommand =
  | { type: 'start' }
  | { type: 'skill'; skillId: string; args?: string }
  | { type: 'reload' }
  | { type: 'autoApprove'; value: boolean }
  | { type: 'dock' }
  | { type: 'bounds'; bounds: WindowBounds }
  | { type: 'agentsOpen'; value: boolean }
  | { type: 'goToSession'; sessionId: string }
  | { type: 'closeSession'; sessionId: string };

export interface CommandEnvelope {
  nonce: number;
  command: AgentWindowCommand;
  /** The session the detached window was showing when the command was sent. */
  sessionId: string | null;
}

/** Commands that act on the focused session. The rest are window-wide. */
export function isSessionScoped(command: AgentWindowCommand): boolean {
  return command.type === 'start' || command.type === 'skill'
    || command.type === 'reload' || command.type === 'autoApprove';
}

/** The refusal for a session-scoped command whose session is no longer the focused one. */
export function sessionScopeError(
  envelope: CommandEnvelope,
  activeSessionId: string | null,
): string | null {
  if (!isSessionScoped(envelope.command)) return null;
  if (!envelope.sessionId || !activeSessionId) return 'no session is focused';
  if (envelope.sessionId !== activeSessionId) return 'the focused session changed';
  return null;
}

export interface CommandDone {
  nonce: number;
  error: string | null;
}

// ── Persisted layout ──────────────────────────────────────────────────────────

const DETACHED_KEY = 'wb.agentDetached';
const BOUNDS_KEY = 'wb.agentWindowBounds';
const AGENTS_KEY = 'wb.agentsSidebarOpen';
const AGENTS_WIDTH_KEY = 'wb.agentsSidebarWidth';

/** Logical pixels, as the window API takes them. */
export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const MIN_WINDOW_WIDTH = 360;
export const MIN_WINDOW_HEIGHT = 400;
export const DEFAULT_WINDOW_SIZE = { width: 560, height: 800 };

/** A saved bounds record, or null. A window below the minimum size is rejected. */
export function parseBounds(raw: string | null): WindowBounds | null {
  if (!raw) return null;
  let v: unknown;
  try {
    v = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!v || typeof v !== 'object') return null;
  const b = v as Record<string, unknown>;
  const nums = [b.x, b.y, b.width, b.height];
  if (!nums.every((n) => typeof n === 'number' && Number.isFinite(n))) return null;
  const width = Math.round(b.width as number);
  const height = Math.round(b.height as number);
  if (width < MIN_WINDOW_WIDTH || height < MIN_WINDOW_HEIGHT) return null;
  return { x: Math.round(b.x as number), y: Math.round(b.y as number), width, height };
}

export function readBounds(): WindowBounds | null {
  try {
    return parseBounds(localStorage.getItem(BOUNDS_KEY));
  } catch {
    return null;
  }
}

export function writeBounds(b: WindowBounds): void {
  try {
    localStorage.setItem(BOUNDS_KEY, JSON.stringify(b));
  } catch {
    /* ignore */
  }
}

export function readDetached(): boolean {
  try {
    return localStorage.getItem(DETACHED_KEY) === 'true';
  } catch {
    return false;
  }
}

export function writeDetached(v: boolean): void {
  try {
    localStorage.setItem(DETACHED_KEY, String(v));
  } catch {
    /* ignore */
  }
}

export function readAgentsSidebar(): boolean {
  try {
    return localStorage.getItem(AGENTS_KEY) === 'true';
  } catch {
    return false;
  }
}

export function writeAgentsSidebar(v: boolean): void {
  try {
    localStorage.setItem(AGENTS_KEY, String(v));
  } catch {
    /* ignore */
  }
}

export function readAgentsWidth(): number {
  try {
    return clampAgentsWidth(Number(localStorage.getItem(AGENTS_WIDTH_KEY)));
  } catch {
    return clampAgentsWidth(null);
  }
}

export function writeAgentsWidth(w: number): void {
  try {
    localStorage.setItem(AGENTS_WIDTH_KEY, String(w));
  } catch {
    /* ignore */
  }
}
