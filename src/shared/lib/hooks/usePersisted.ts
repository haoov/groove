import { useCallback, useRef, useState } from 'react';

/** How a value is turned into a localStorage string and back. */
export interface Codec<T> {
  parse: (raw: string) => T;
  write: (value: T) => string;
}

export const STRING_CODEC: Codec<string> = { parse: (raw) => raw, write: (v) => v };

export const SET_CODEC: Codec<Set<string>> = {
  parse: (raw) => new Set(JSON.parse(raw) as string[]),
  write: (v) => JSON.stringify([...v]),
};

export const NUMBER_CODEC: Codec<number> = { parse: Number, write: String };

export function readPersisted<T>(key: string, fallback: T, codec: Codec<T>): T {
  try {
    const raw = localStorage.getItem(key);
    return raw === null ? fallback : codec.parse(raw);
  } catch {
    return fallback;
  }
}

export function writePersisted<T>(key: string, value: T, codec: Codec<T>): void {
  try { localStorage.setItem(key, codec.write(value)); } catch { /* ignore */ }
}

/** `useState` that seeds from localStorage and writes every update back. */
export function usePersisted<T>(key: string, fallback: T, codec: Codec<T>) {
  const [value, setValue] = useState<T>(() => readPersisted(key, fallback, codec));
  const latest = useRef(value);
  latest.current = value;

  const set = useCallback((next: T | ((prev: T) => T)) => {
    const v = typeof next === 'function' ? (next as (prev: T) => T)(latest.current) : next;
    latest.current = v;
    writePersisted(key, v, codec);
    setValue(v);
  }, [key, codec]);

  return [value, set] as const;
}
