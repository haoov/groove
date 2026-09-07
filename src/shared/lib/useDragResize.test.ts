import { describe, expect, it } from 'vitest';
import { clampSize, readStoredSize } from './useDragResize';

describe('clampSize', () => {
  it('holds a drag inside the range', () => {
    expect(clampSize(200, 320, 900)).toBe(320);
    expect(clampSize(1200, 320, 900)).toBe(900);
    expect(clampSize(460.5, 320, 900)).toBe(460.5);
  });

  it('falls back to the minimum for anything that is not a size', () => {
    expect(clampSize(NaN, 320, 900)).toBe(320);
    expect(clampSize(Infinity, 320, 900)).toBe(320);
  });
});

describe('readStoredSize', () => {
  const store = new Map<string, string>();
  globalThis.localStorage = {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => { store.set(k, v); },
  } as unknown as Storage;

  it('reads a stored size and rejects one under the minimum', () => {
    store.set('wb.pane', '500');
    expect(readStoredSize('wb.pane', 320, 460)).toBe(500);
    store.set('wb.pane', '10');
    expect(readStoredSize('wb.pane', 320, 460)).toBe(460);
  });

  it('falls back with nothing stored', () => {
    store.delete('wb.pane');
    expect(readStoredSize('wb.pane', 320, 460)).toBe(460);
    store.set('wb.pane', 'wide');
    expect(readStoredSize('wb.pane', 320, 460)).toBe(460);
  });
});
