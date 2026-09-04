import { describe, expect, it } from 'vitest';
import { MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, parseBounds } from './agentWindow';

describe('saved agent window bounds', () => {
  it('round-trips a record and rounds to whole pixels', () => {
    const raw = JSON.stringify({ x: 10.4, y: -20.6, width: 600.5, height: 800 });
    expect(parseBounds(raw)).toEqual({ x: 10, y: -21, width: 601, height: 800 });
  });

  it('is null for nothing, garbage and non-objects', () => {
    expect(parseBounds(null)).toBeNull();
    expect(parseBounds('')).toBeNull();
    expect(parseBounds('{not json')).toBeNull();
    expect(parseBounds('42')).toBeNull();
    expect(parseBounds('null')).toBeNull();
  });

  it('is null when a field is missing or not a finite number', () => {
    expect(parseBounds(JSON.stringify({ x: 0, y: 0, width: 600 }))).toBeNull();
    expect(parseBounds(JSON.stringify({ x: '0', y: 0, width: 600, height: 800 }))).toBeNull();
    expect(parseBounds(JSON.stringify({ x: 0, y: 0, width: Infinity, height: 800 }))).toBeNull();
  });

  it('is null below the minimum size', () => {
    const small = { x: 0, y: 0, width: MIN_WINDOW_WIDTH - 1, height: MIN_WINDOW_HEIGHT };
    expect(parseBounds(JSON.stringify(small))).toBeNull();
    const short = { x: 0, y: 0, width: MIN_WINDOW_WIDTH, height: MIN_WINDOW_HEIGHT - 1 };
    expect(parseBounds(JSON.stringify(short))).toBeNull();
    const ok = { x: 0, y: 0, width: MIN_WINDOW_WIDTH, height: MIN_WINDOW_HEIGHT };
    expect(parseBounds(JSON.stringify(ok))).toEqual(ok);
  });
});
