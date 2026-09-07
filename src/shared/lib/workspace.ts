// Pure lookups shared across workspace components. Plain functions, not hooks.

import type { Worktree, Mr, Annotation, MrThread, DiffResult } from '../ipc/ipc';

/** The worktree for a repo. */
export const worktreeFor = (worktrees: Worktree[], repoId: string | null | undefined) =>
  repoId ? worktrees.find((w) => w.repo_id === repoId) : undefined;

/** The worktree git ops target for a repo; the session's selected worktree when it belongs to that repo. */
export const activeWorktreeFor = (
  worktrees: Worktree[],
  repoId: string | null | undefined,
  activeWorktreeId: string | null | undefined,
) => {
  if (!repoId) return undefined;
  const selected = activeWorktreeId ? worktrees.find((w) => w.id === activeWorktreeId) : undefined;
  return selected?.repo_id === repoId ? selected : worktrees.find((w) => w.repo_id === repoId);
};

/** The diff summary entry for one worktree. The repo match is the fallback while a worktree provisions. */
export const repoDiffFor = (diff: DiffResult | null | undefined, worktreeId: string | undefined, repoId: string) =>
  (worktreeId ? diff?.repos.find((r) => r.worktree_id === worktreeId) : undefined)
    ?? diff?.repos.find((r) => r.repo_id === repoId);

/** The MR attached to a worktree, if any. */
export const mrForWorktree = (mrs: Mr[], worktreeId: string | undefined) =>
  worktreeId ? mrs.find((m) => m.worktree_id === worktreeId) ?? null : null;

/** Open annotations for one file. */
export const openFileAnnotations = (annotations: Annotation[], repoId: string, filePath: string) =>
  annotations.filter((a) => a.repo_id === repoId && a.file_path === filePath && a.status === 'open');

/** MR threads anchored in one file. */
export const fileThreads = (threads: MrThread[], filePath: string) =>
  threads.filter((d) => d.notes?.[0]?.position?.new_path === filePath);
