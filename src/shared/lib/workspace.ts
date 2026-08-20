// Small pure lookups shared across workspace components, replacing the repeated
// inline `activeWorktrees.find(...)`, `mrs.find(...)`, and per-file annotation/
// thread filters. Plain functions (not hooks) so they work inside render loops.

import type { Worktree, Mr, Annotation, MrThread } from '../ipc/ipc';

/** The worktree for a repo (the one git ops target). */
export const worktreeFor = (worktrees: Worktree[], repoId: string | null | undefined) =>
  repoId ? worktrees.find((w) => w.repo_id === repoId) : undefined;

/** The worktree git ops target for a repo, honoring the session's selected
 *  worktree when it belongs to that repo (a repo can hold several worktrees). */
export const activeWorktreeFor = (
  worktrees: Worktree[],
  repoId: string | null | undefined,
  activeWorktreeId: string | null | undefined,
) => {
  if (!repoId) return undefined;
  const selected = activeWorktreeId ? worktrees.find((w) => w.id === activeWorktreeId) : undefined;
  return selected?.repo_id === repoId ? selected : worktrees.find((w) => w.repo_id === repoId);
};

/** The MR attached to a worktree, if any. */
export const mrForWorktree = (mrs: Mr[], worktreeId: string | undefined) =>
  worktreeId ? mrs.find((m) => m.worktree_id === worktreeId) ?? null : null;

/** Open annotations for one file. */
export const openFileAnnotations = (annotations: Annotation[], repoId: string, filePath: string) =>
  annotations.filter((a) => a.repo_id === repoId && a.file_path === filePath && a.status === 'open');

/** MR threads anchored in one file. */
export const fileThreads = (threads: MrThread[], filePath: string) =>
  threads.filter((d) => d.notes?.[0]?.position?.new_path === filePath);
