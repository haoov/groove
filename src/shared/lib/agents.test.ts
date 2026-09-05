import { describe, expect, it } from 'vitest';
import {
  AGENTS_SIDEBAR_DEFAULT, AGENTS_SIDEBAR_MAX, AGENTS_SIDEBAR_MIN,
  agentLine, buildAgentRows, clampAgentsWidth, waitingCount,
} from './agents';
import { mrRef } from './forge';
import type { AgentActivity } from '../ipc/ipc';

const activity = (state: AgentActivity['state'], extra: Partial<AgentActivity> = {}): AgentActivity => ({
  task_id: 'T-1', state, tool: null, last_message: null, since: 0, ...extra,
});

const task = (id: string, short_id: string, title = `Task ${short_id}`) => ({
  id, kind: 'task' as const, title: short_id, mrs: [],
  task: { short_id, title, status: 'Ready' } as never,
});

describe('agentLine', () => {
  it('names the tool a waiting agent is blocked on', () => {
    expect(agentLine(activity('waiting', { tool: { name: 'Write', detail: 'a/b.rs' } }))).toBe('waiting · Write(a/b.rs)');
    expect(agentLine(activity('waiting'))).toBe('waiting on you');
  });

  it('shows the tool in flight, or the closing line once idle', () => {
    expect(agentLine(activity('working', { tool: { name: 'Bash', detail: null } }))).toBe('Bash');
    expect(agentLine(activity('working'))).toBe('working…');
    expect(agentLine(activity('idle', { last_message: 'Done.' }))).toBe('idle · Done.');
    expect(agentLine(activity('idle'))).toBe('idle');
  });
});

describe('clampAgentsWidth', () => {
  it('keeps a drag inside the range and rounds it', () => {
    expect(clampAgentsWidth(AGENTS_SIDEBAR_MIN - 50)).toBe(AGENTS_SIDEBAR_MIN);
    expect(clampAgentsWidth(AGENTS_SIDEBAR_MAX + 50)).toBe(AGENTS_SIDEBAR_MAX);
    expect(clampAgentsWidth(300.6)).toBe(301);
  });

  it('falls back to the default for anything that is not a width', () => {
    expect(clampAgentsWidth(NaN)).toBe(AGENTS_SIDEBAR_DEFAULT);
    expect(clampAgentsWidth('280')).toBe(AGENTS_SIDEBAR_DEFAULT);
    expect(clampAgentsWidth(null)).toBe(AGENTS_SIDEBAR_DEFAULT);
  });
});

describe('buildAgentRows', () => {
  const sessions = { a: task('a', 'T-1'), b: task('b', 'T-2'), c: task('c', 'T-3') };

  it('puts waiting agents first and keeps session order otherwise', () => {
    const rows = buildAgentRows(sessions, ['a', 'b', 'c'], 'a', {
      'T-1': activity('working'),
      'T-3': activity('waiting'),
    });
    expect(rows.map((r) => r.taskId)).toEqual(['T-3', 'T-1', 'T-2']);
    expect(rows.find((r) => r.taskId === 'T-1')?.active).toBe(true);
    expect(waitingCount(rows)).toBe(1);
  });

  it('skips a session id with no session behind it', () => {
    expect(buildAgentRows(sessions, ['a', 'gone'], null, {})).toHaveLength(1);
  });

  it('labels a review by its MR number', () => {
    const review = {
      id: 'r', kind: 'review' as const, title: 'long review title', task: null,
      mrs: [{ platform: 'gitlab', remote_id: '42' } as never],
    };
    const [row] = buildAgentRows({ r: review }, ['r'], null, {});
    expect(row.idLabel).toBe(mrRef('gitlab' as never, '42' as never));
    expect(row.title).toBe('long review title');
  });
});
