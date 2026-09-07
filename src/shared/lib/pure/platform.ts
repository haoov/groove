export type Platform = 'macos' | 'linux' | 'windows';

let current: Platform = 'linux';

export function platform(): Platform {
  return current;
}

export function isMac(): boolean {
  return current === 'macos';
}

/** Set by `actions/initPlatform`, and directly by tests. */
export function setPlatform(p: Platform): void {
  current = p;
}
