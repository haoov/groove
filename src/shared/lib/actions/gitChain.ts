// "Commit & Push" chaining. Post the push only after the commit's confirmation resolves approved.

const pendingPush = new Map<string, string>(); // commit confirmation id → worktree id

export function registerCommitPush(confirmationId: string, worktreeId: string) {
  pendingPush.set(confirmationId, worktreeId);
}

/** Consumes the chain entry for a resolved commit, whatever the outcome. */
export function takeCommitPush(confirmationId: string): string | undefined {
  const wt = pendingPush.get(confirmationId);
  pendingPush.delete(confirmationId);
  return wt;
}
