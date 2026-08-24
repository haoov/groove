import { useMemo } from 'react';
import { useStore } from '../shared/store';

// The values the filter can suggest. Everything here comes from the rows already
// loaded, so a suggestion can never match nothing. Booleans and kinds are fixed
// — they are the field's whole domain, not a sample of it.

const add = (into: Map<string, Set<string>>, key: string, value: string | null | undefined) => {
  if (!value) return;
  const set = into.get(key) ?? new Set<string>();
  set.add(value);
  into.set(key, set);
};

export function useFilterValues(): Record<string, string[]> {
  const snapshot = useStore((s) => s.homeSnapshot);
  const tasks = useStore((s) => s.tasks);
  const reviews = useStore((s) => s.reviewQueue);

  return useMemo(() => {
    const m = new Map<string, Set<string>>([
      ['kind', new Set(['task', 'explorer', 'review'])],
      ['approved', new Set(['true', 'false'])],
      ['draft', new Set(['true', 'false'])],
    ]);
    for (const e of snapshot ?? []) {
      add(m, 'status', e.status);
      add(m, 'priority', e.priority);
      add(m, 'provider', e.provider);
      for (const r of e.repos) {
        add(m, 'repo', r.project);
        add(m, 'branch', r.branch);
      }
    }
    for (const t of tasks) {
      add(m, 'status', t.status);
      add(m, 'priority', t.priority);
      add(m, 'provider', t.provider);
    }
    for (const mr of reviews ?? []) {
      add(m, 'provider', mr.platform);
      add(m, 'repo', mr.project_full);
      add(m, 'branch', mr.source_branch);
      add(m, 'owner', mr.author);
      add(m, 'author', mr.author);
    }
    const out: Record<string, string[]> = {};
    for (const [k, set] of m) out[k] = [...set].sort((a, b) => a.localeCompare(b));
    return out;
  }, [snapshot, tasks, reviews]);
}
