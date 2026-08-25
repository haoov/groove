// Shared, presentation-free logic for Home: opening sessions, ranking, labels.
import { mrRef } from '../shared/lib/forge';

import { invoke } from '../shared/ipc/invoke';
import { useStore, sessionActions } from '../shared/store';
import { ciGroup } from '../shared/lib/mr';
import type { HomeEntry, HomeRepo } from '../shared/ipc/ipc';

export const openTask = (shortId: string) =>
  invoke('open_task', { shortId }).catch((e) => useStore.getState().setLastError(String(e)));

/** Open a session and land directly on one repo's "all changes" tab when the
 *  session is already mounted (a cold open can only route to the session). */
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

/** Rolled-up state: orders Live, and fills the FOLDED row's summary chips. */
export function summarize(entry: HomeEntry) {
  let dirty = 0;
  let added = 0;
  let deleted = 0;
  let ahead = 0;
  let behind = 0;
  let mrs = 0;
  let unresolved = 0;
  let ciFail = false;
  let attention = false;
  for (const r of entry.repos) {
    dirty += r.modified + r.staged;
    added += r.added;
    deleted += r.deleted;
    ahead += r.ahead;
    behind += r.behind;
    if (r.mr) {
      mrs += 1;
      unresolved += r.mr.unresolved;
      if (r.mr.ci && ciGroup(r.mr.ci) === 'fail') { ciFail = true; attention = true; }
    }
    if (r.conflicted > 0 || r.missing) attention = true;
  }
  return { dirty, added, deleted, ahead, behind, mrs, unresolved, ciFail, attention };
}

export const KIND_LABEL = { task: 'task', explorer: 'expl', review: 'review' } as const;

/** The key column: always filled so the title column lines up across kinds. */
/** Where a row came from. A review keeps its MR number: that is the only place it
 *  is shown, and the sigil already names the forge. */
export function rowProvider(entry: HomeEntry): string {
  if (entry.kind === 'review') {
    const mr = entry.repos.find((r) => r.mr)?.mr;
    if (mr) return mrRef(mr.platform, mr.remote_id);
  }
  if (entry.kind === 'explorer') return 'local';
  return entry.provider ?? '—';
}

// priorityRank / priorityLabel moved to shared/lib/taskStatus (used across features).
export { priorityLabel, priorityRank } from '../shared/lib/taskStatus';
