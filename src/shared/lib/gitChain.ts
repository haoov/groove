// "Commit & Push" chaining. The push must be posted only AFTER the
// commit's confirmation resolves approved — posting both at once let the user
// deny the commit and still approve a push of a not-yet-committed tree.

const pendingPush = new Map<string, string>(); // commit confirmation id → worktree id

export function registerCommitPush(confirmationId: string, worktreeId: string) {
  pendingPush.set(confirmationId, worktreeId);
}

/** Consume the chain entry for a resolved commit (whatever the outcome). */
export function takeCommitPush(confirmationId: string): string | undefined {
  const wt = pendingPush.get(confirmationId);
  pendingPush.delete(confirmationId);
  return wt;
}
