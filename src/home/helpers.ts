// Shared, presentation-free logic for Home: opening sessions, ranking, labels.

import { invoke } from '@tauri-apps/api/core';
import { useStore, sessionActions } from '../shared/store';
import { ciGroup } from '../notes/MrThreads';
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

/** Rolled-up state — used only for ordering Live now that the row itself shows
 *  no aggregates (each repo reports its own state below). */
export function summarize(entry: HomeEntry) {
  let dirty = 0;
  let attention = false;
  for (const r of entry.repos) {
    dirty += r.modified + r.staged;
    if (r.conflicted > 0 || r.missing) attention = true;
    if (r.mr?.ci && ciGroup(r.mr.ci) === 'fail') attention = true;
  }
  return { dirty, attention };
}

export const KIND_LABEL = { task: 'task', explorer: 'expl', review: 'review' } as const;

/** The key column: always filled so the title column lines up across kinds. */
export function rowKey(entry: HomeEntry): string {
  if (entry.kind === 'review') {
    const iid = entry.repos.find((r) => r.mr)?.mr?.remote_id;
    if (iid) return `!${iid}`;
  }
  // The EXPL badge already carries the kind — the prefix would only widen the
  // key column for every other row.
  if (entry.kind === 'explorer') return entry.short_id.replace(/^explorer-/, '');
  return entry.short_id;
}

/** Compact label for the priority pill; '' when the task has no priority
 *  (which must stay distinct from "Low" — both rank 3). */
export function priorityLabel(priority: string | null): string {
  if (!priority) return '';
  return ['urg', 'high', 'med', 'low'][priorityRank(priority)] ?? '';
}

export function priorityRank(priority: string | null): number {
  const s = (priority ?? '').toLowerCase();
  if (s.includes('p0') || s.includes('urgent') || s.includes('critical') || s.includes('blocker')) return 0;
  if (s.includes('p1') || s.includes('high')) return 1;
  if (s.includes('p2') || s.includes('medium') || s.includes('normal')) return 2;
  return 3;
}
