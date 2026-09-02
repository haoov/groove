import { useEffect, useState } from 'react';
import { GitBranch } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import type { OriginBranches } from '../shared/ipc/ipc';
import { Combobox } from '../shared/ui/Combobox';
import { Highlighted } from '../shared/lib/match';

/**
 * Picks a worktree's base branch — `Worktree::base_ref`, later the MR target.
 * The list comes from origin, so it only offers what provisioning accepts.
 */

type State =
  | { phase: 'loading' }
  | { phase: 'ready'; branches: string[]; fallback: string | null }
  | { phase: 'failed'; error: string };

const LOADING: State = { phase: 'loading' };
/** Stable identity — a fresh [] each render would rebuild the ranking. */
const NO_BRANCHES: string[] = [];

/**
 * Origin's branches for one repo, fetched once per repo id. The answer carries
 * the id it belongs to, so a previous repo's list is never shown as this one's.
 */
export function useOriginBranches(repoId: string | undefined): State {
  const [loaded, setLoaded] = useState<{ repoId: string; state: State } | null>(null);

  useEffect(() => {
    if (!repoId) return;
    let live = true;
    invoke<OriginBranches>('list_origin_branches', { repoId })
      .then((r) => {
        if (live) {
          setLoaded({ repoId, state: { phase: 'ready', branches: r.branches, fallback: r.default_branch } });
        }
      })
      .catch((e) => {
        if (live) setLoaded({ repoId, state: { phase: 'failed', error: String(e) } });
      });
    return () => { live = false; };
  }, [repoId]);

  return loaded && loaded.repoId === repoId ? loaded.state : LOADING;
}

/** Combobox over `state`'s branches. `value` is '' for "the repo default". */
export function BranchPicker({
  state, value, onChange,
}: {
  state: State;
  value: string;
  onChange: (branch: string) => void;
}) {
  const branches = state.phase === 'ready' ? state.branches : NO_BRANCHES;
  const fallback = state.phase === 'ready' ? state.fallback : null;

  const placeholder =
    state.phase === 'loading' ? 'Reading origin…'
      : state.phase === 'failed' ? 'origin unreachable'
        : fallback ?? 'pick a branch';

  return (
    <>
      <Combobox
        items={branches}
        toText={(b) => b}
        value={value}
        onPick={onChange}
        onClear={() => onChange('')}
        icon={GitBranch}
        placeholder={placeholder}
        disabled={state.phase !== 'ready'}
        emptyLabel="No branch matches"
        inputClassName="mono"
        renderItem={(b, ranges) => (
          <>
            <span className="cbx-name mono"><Highlighted text={b} ranges={ranges} /></span>
            {b === fallback && <span className="cbx-tag">default</span>}
          </>
        )}
      />
      {state.phase === 'failed' && (
        <span className="firstrun-hint">Could not read origin: {state.error}</span>
      )}
    </>
  );
}
