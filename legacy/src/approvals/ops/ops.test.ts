import { describe, it, expect } from 'vitest';
import { OP, type OpType } from '../../shared/ipc/ops';
import { OP_SPECS, opSpec } from './index';
import { repoName, str } from './spec';

type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

describe('OP_SPECS', () => {
  // A row for an unknown op, or an op with no row, fails to compile here.
  it('covers exactly the OpType union', () => {
    const covered: Same<keyof typeof OP_SPECS, OpType> = true;
    expect(covered).toBe(true);
  });

  it('holds exactly one entry per op type', () => {
    expect(Object.keys(OP_SPECS).sort()).toEqual(Object.values(OP).slice().sort());
  });

  it('gives every entry a label, an icon and a view', () => {
    for (const [op, spec] of Object.entries(OP_SPECS)) {
      expect(spec.label, op).toBeTruthy();
      expect(spec.icon, op).toBeTruthy();
      expect(typeof spec.View, op).toBe('function');
    }
  });

  it('has no spec for an op the backend has not shipped yet', () => {
    expect(opSpec('git.bisect')).toBeUndefined();
    expect(opSpec(OP.GIT_COMMIT)?.label).toBe('Git commit');
  });
});

describe('seeding', () => {
  it('seeds a commit message from the payload', () => {
    expect(OP_SPECS[OP.GIT_COMMIT].seed?.({ message: 'fix: thing' })).toEqual({ message: 'fix: thing' });
  });

  it('seeds an empty MR description with the required headings', () => {
    const seeded = OP_SPECS[OP.MR_CREATE].seed?.({ title: 'x' });
    expect(seeded?.title).toBe('x');
    expect(seeded?.description).toBe('## What\n\n\n## Why\n\n');
  });

  it('seeds both task-create ops the same way', () => {
    const p = { title: 't', body_markdown: 'b' };
    expect(OP_SPECS[OP.TASK_CREATE_FROM_EXPLORER].seed?.(p)).toEqual(OP_SPECS[OP.TASK_CREATE].seed?.(p));
  });

  it('leaves an op with nothing editable unseeded', () => {
    expect(OP_SPECS[OP.GIT_PUSH].seed).toBeUndefined();
  });
});

describe('validation', () => {
  it('blocks an MR with no title', () => {
    expect(OP_SPECS[OP.MR_CREATE].validate?.({ title: '   ' })).toMatch(/title/);
    expect(OP_SPECS[OP.MR_CREATE].validate?.({ title: 'feat: x' })).toBeNull();
  });

  it('leaves every other op unvalidated', () => {
    const validated = Object.entries(OP_SPECS).filter(([, s]) => s.validate);
    expect(validated.map(([op]) => op)).toEqual([OP.MR_CREATE]);
  });
});

describe('payload helpers', () => {
  it('reads a string field, empty when absent', () => {
    expect(str({ branch: 'main' }, 'branch')).toBe('main');
    expect(str({}, 'branch')).toBe('');
  });

  it('prefers the payload repo over the worktree path', () => {
    expect(repoName({ repo: 'mayo', worktree_path: '/w/mayo/fix/x' })).toBe('mayo');
  });

  it('falls back to the last path segment', () => {
    expect(repoName({ worktree_path: '/w/mayo/fix/parser/' })).toBe('parser');
    expect(repoName({})).toBe('');
  });
});
