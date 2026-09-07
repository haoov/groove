// Shared, presentation-free logic for Home: opening sessions, ranking, labels.
import { mrRef } from '../shared/lib/pure/forge';

import { invoke } from '../shared/ipc/invoke';
import { useStore, sessionActions } from '../shared/store';
import type { HomeEntry, HomeRepo } from '../shared/ipc/ipc';

export const openTask = (shortId: string) =>
  invoke('open_task', { shortId }).catch((e) => useStore.getState().setLastError(e));

/** Opens a session on one repo's "all changes" tab when mounted; a cold open routes to the session. */
export function openRepo(entry: HomeEntry, repo: HomeRepo) {
  const st = useStore.getState();
  const sid = st.sessionOrder.find((id) => st.sessions[id]?.task?.short_id === entry.short_id);
  if (!sid) {
    openTask(entry.short_id);
    return;
  }
  const a = sessionActions(sid);
  a.setActiveRepoId(repo.repo_id);
  a.openTab({ repoId: repo.repo_id, filePath: '', view: 'diff', kind: 'changes', label: 'All changes' });
  st.focusSession(sid);
}

/** A provisioned worktree whose directory is gone sorts first in Live. */
export function needsAttention(entry: HomeEntry): boolean {
  return entry.repos.some((r) => r.missing);
}

export const KIND_LABEL = { task: 'task', explorer: 'expl', review: 'review' } as const;

/** Where a row came from. A review shows its MR reference. */
export function rowProvider(entry: HomeEntry): string {
  if (entry.kind === 'review') {
    const mr = entry.repos.find((r) => r.mr)?.mr;
    if (mr) return mrRef(mr.platform, mr.remote_id);
  }
  if (entry.kind === 'explorer') return 'local';
  return entry.provider ?? '—';
}

export { priorityLabel, priorityRank } from '../shared/lib/pure/taskStatus';
