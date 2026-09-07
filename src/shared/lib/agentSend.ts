// Sending text to a session's agent — writing bytes to its PTY.

import { invoke } from '../ipc/invoke';
import { bytesToB64 } from './ptyRegistry';
import { useStore } from '../store';
import { skillCommand, trimForPty } from './skills';

/** Give up waiting for a cold agent and send. */
const READY_TIMEOUT_MS = 25_000;
const READY_POLL_MS = 200;
/** Grace after a cold start for the input box to mount. */
const READY_SETTLE_MS = 600;
/** Gap between the prompt text and the Enter that submits it. */
const SUBMIT_DELAY_MS = 150;
/** Enter on a TTY is 0x0D, not 0x0A. */
const CARRIAGE_RETURN = 13;

/** The session's most recent agent PTY, or null when it has none running. */
export function agentPtyFor(sessionKey: string): string | null {
  const sess = useStore.getState().sessions[sessionKey];
  if (!sess) return null;
  const agents = sess.ptySessions.filter((p) => p.ptyType === 'agent');
  return agents[agents.length - 1]?.sessionId ?? null;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Waits for the agent's `SessionStart` hook; on timeout, sends anyway. */
async function waitUntilReady(taskId: string): Promise<void> {
  const deadline = Date.now() + READY_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (useStore.getState().agentActivity[taskId]) {
      await sleep(READY_SETTLE_MS);
      return;
    }
    await sleep(READY_POLL_MS);
  }
}

/** The session's agent PTY, started if needed. Output buffers until a host attaches. */
export async function ensureAgentSession(
  sessionKey: string,
  opts?: { waitReady?: boolean },
): Promise<string> {
  const sess = useStore.getState().sessions[sessionKey];
  const taskId = sess?.task?.short_id;
  if (!sess || !taskId) throw new Error('that session has no task to talk about');

  const existing = agentPtyFor(sessionKey);
  if (existing) return existing;

  const pty = await invoke<string>('start_agent_session', { taskId });
  if (opts?.waitReady) await waitUntilReady(taskId);
  return pty;
}

/** Sends a prompt: the text, a pause, then a lone CR. A trailing "\n" in the same write does not submit. */
export async function sendToAgent(sessionKey: string, text: string): Promise<void> {
  const pty = await ensureAgentSession(sessionKey, { waitReady: true });
  const write = (bytes: Uint8Array) => invoke('write_pty', { sessionId: pty, dataB64: bytesToB64(bytes) });

  const body = trimForPty(text);
  await write(new TextEncoder().encode(body));

  // Let the TUI finish the paste before the CR.
  await sleep(SUBMIT_DELAY_MS);
  await write(new Uint8Array([CARRIAGE_RETURN]));
}

/** Invokes a skill on a session's agent: `/groove:start-task `, then Enter. */
export function sendSkill(sessionKey: string, skillId: string, args?: string): Promise<void> {
  return sendToAgent(sessionKey, skillCommand(skillId, args));
}

/** Restarts the agent; `--resume` keeps the conversation. */
export async function reloadAgent(sessionKey: string): Promise<void> {
  const pty = agentPtyFor(sessionKey);
  if (pty) await invoke('stop_agent_session', { sessionId: pty });
  await ensureAgentSession(sessionKey);
  await useStore.getState().loadSkills();
  useStore.getState().setSkillsStale(false);
}
