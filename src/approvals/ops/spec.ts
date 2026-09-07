// ── What one approval op contributes to the dialog ────────────────────────────

import type { FC } from 'react';
import type { LucideIcon } from 'lucide-react';
import type { OpType } from '../../shared/ipc/ops';

export type Payload = Record<string, unknown>;
export type Edits = Record<string, string>;

export interface OpViewProps {
  payload: Payload;
  edits: Edits;
  setField: (key: string, value: string) => void;
}

export interface OpSpec {
  label: string;
  icon: LucideIcon;
  /** Editable fields, seeded from the payload when the request opens. */
  seed?: (payload: Payload) => Edits;
  /** Why the request cannot be approved yet, else null. */
  validate?: (edits: Edits) => string | null;
  View: FC<OpViewProps>;
}

/** The ops of one family, e.g. `OpsOf<'git.'>`. */
export type OpsOf<Prefix extends string> = Record<Extract<OpType, `${Prefix}${string}`>, OpSpec>;

/** A payload's string field, empty when absent. */
export const str = (payload: Payload, key: string): string =>
  (payload[key] as string | undefined) ?? '';

// The path's last segment is the branch leaf, not the repo: a fallback only.
export function repoName(payload: Payload): string {
  const repo = payload.repo;
  if (typeof repo === 'string' && repo) return repo;
  const path = str(payload, 'worktree_path');
  return path.split('/').filter(Boolean).pop() ?? path;
}
