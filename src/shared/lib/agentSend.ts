// Sending text to a session's agent, from anywhere in the app.
//
// The agent is a PTY running Claude Code, so "sending a prompt" is writing bytes
// to its terminal — the same thing as typing into it. Two consequences shape this
// module: the text must end in a newline to be submitted, and it must not arrive
// before Claude's input is ready to receive it.

import { invoke } from '../ipc/invoke';
import { bytesToB64 } from './ptyRegistry';
import { useStore } from '../store';

/** Give up waiting for a cold agent to report in and just send. */
const READY_TIMEOUT_MS = 25_000;
const READY_POLL_MS = 200;
/** Grace after a cold start so the input box is mounted, not just the process. */
const READY_SETTLE_MS = 600;
/** Gap between the prompt text and the Enter that submits it. */
const SUBMIT_DELAY_MS = 150;
/** What the Enter key actually sends on a TTY — 0x0D, not 0x0A. */
const CARRIAGE_RETURN = 13;

/** The session's most recent agent PTY, or null when it has none running. */
export function agentPtyFor(sessionKey: string): string | null {
  const sess = useStore.getState().sessions[sessionKey];
  if (!sess) return null;
  const agents = sess.ptySessions.filter((p) => p.ptyType === 'agent');
  return agents[agents.length - 1]?.sessionId ?? null;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * Wait for a freshly started agent to be ready for input.
 *
 * Its `SessionStart` hook firing is the signal — that's Claude telling us it is
 * up, which beats guessing at a delay (a cold start is fast, a `--resume` of a
 * long conversation is not). If hooks never arrive (curl missing, older agent),
 * we fall through on the timeout and send anyway rather than silently dropping
 * the user's prompt.
 */
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

/**
 * The session's agent PTY, started if it isn't running yet.
 *
 * Starting does NOT open the agent tab: PTY output is buffered until a terminal
 * host registers (useIpc), so nothing is lost if no surface is showing it yet.
 * When `waitReady` is set, this also waits for the agent to be able to take
 * input — only needed before writing to it.
 */
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

/**
 * Send a prompt to a session's agent, starting one if it isn't running.
 *
 * Submitting takes two writes, not one. A canned prompt is multi-line, and Claude
 * Code treats the newlines inside a fast write as newlines in its input box — so a
 * trailing "\n" just left the whole prompt sitting there unsent. The text goes
 * first, then a lone carriage return after a short pause, which the TUI reads as
 * Enter. Verified against a real PTY: without the split the draft never submits.
 */
export async function sendToAgent(sessionKey: string, text: string): Promise<void> {
  const pty = await ensureAgentSession(sessionKey, { waitReady: true });
  const write = (bytes: Uint8Array) => invoke('write_pty', { sessionId: pty, dataB64: bytesToB64(bytes) });

  // Trailing newlines would become blank lines in the draft. A trailing SPACE
  // survives: `sendSkill` needs it to close Claude's slash menu before the CR.
  const body = text.replace(/[^\S ]+$/, '');
  await write(new TextEncoder().encode(body));

  // Long enough that the TUI has finished handling the paste, so the CR arrives as
  // its own keypress instead of being folded into it.
  await sleep(SUBMIT_DELAY_MS);
  await write(new Uint8Array([CARRIAGE_RETURN]));
}

/**
 * Invoke a skill on a session's agent: `/groove:open-mr `, then Enter.
 *
 * The TRAILING SPACE is load-bearing. Typing `/` opens Claude's slash menu, and
 * with the menu open Enter acts on the highlighted row — which is not always the
 * skill we named, since the menu ranks by prefix and past use. A space closes the
 * menu, so the CR submits the literal text. Verified against a real PTY: with two
 * skills sharing a prefix, the menu is still open at the CR without it.
 */
export function sendSkill(sessionKey: string, skillId: string, args?: string): Promise<void> {
  return sendToAgent(sessionKey, `/${skillId} ${args ?? ''}`);
}

/**
 * Restart a session's agent so it loads the skills that are on disk now.
 *
 * `--plugin-dir` is read at launch, so a skill written mid-session is invisible
 * to the agent that wrote it. `start_agent_session` resumes the same Claude
 * conversation, so the restart costs the process, not the thread.
 */
export async function reloadAgent(sessionKey: string): Promise<void> {
  const pty = agentPtyFor(sessionKey);
  if (pty) await invoke('stop_agent_session', { sessionId: pty });
  await ensureAgentSession(sessionKey);
  await useStore.getState().loadSkills();
  useStore.getState().setSkillsStale(false);
}
