import { describe, expect, it } from 'vitest';
import { errorKind, errorText, isNotFound } from './appError';

describe('errorText', () => {
  it('reads an AppError', () => {
    expect(errorText({ kind: 'git', message: 'index.lock exists' })).toBe('index.lock exists');
  });

  it('reads a bare string', () => {
    expect(errorText('worktree is dirty')).toBe('worktree is dirty');
  });

  it('reads an Error', () => {
    expect(errorText(new Error('boom'))).toBe('boom');
  });

  it('reads a plain object with a message', () => {
    expect(errorText({ message: 'no remote' })).toBe('no remote');
  });

  it('falls back to the string form of anything else', () => {
    expect(errorText(undefined)).toBe('undefined');
    expect(errorText(null)).toBe('null');
    expect(errorText(42)).toBe('42');
    expect(errorText({ kind: 'git' })).toBe('[object Object]');
  });
});

describe('errorKind', () => {
  it('returns the kind of an AppError', () => {
    expect(errorKind({ kind: 'not_found', message: 'gone' })).toBe('not_found');
    expect(errorKind({ kind: 'conflict', message: 'already there' })).toBe('conflict');
  });

  it('returns null for anything that is not an AppError', () => {
    expect(errorKind('plain string')).toBeNull();
    expect(errorKind(new Error('boom'))).toBeNull();
    expect(errorKind({ message: 'no kind' })).toBeNull();
    expect(errorKind(undefined)).toBeNull();
  });

  it('narrows not_found only', () => {
    expect(isNotFound({ kind: 'not_found', message: 'gone' })).toBe(true);
    expect(isNotFound({ kind: 'io', message: 'gone' })).toBe(false);
    expect(isNotFound('not found')).toBe(false);
  });
});
