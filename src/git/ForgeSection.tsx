import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  GitPullRequest, GitMerge, GitPullRequestClosed, MessageSquare,
} from 'lucide-react';
import type { Mr } from '../shared/ipc/ipc';
import { ciGroup } from '../shared/lib/mr';

export interface ForgeItem {
  mr: Mr;
  repoId: string;
  repoName: string;
  unresolved: number;
}

function StateIcon({ state }: { state: string }) {
  if (state === 'merged') return <GitMerge size={13} strokeWidth={1.75} />;
  if (state === 'closed') return <GitPullRequestClosed size={13} strokeWidth={1.75} />;
  return <GitPullRequest size={13} strokeWidth={1.75} />;
}

function ForgeRow({
  item, multiRepo, onSelect,
}: {
  item: ForgeItem;
  multiRepo: boolean;
  onSelect: (item: ForgeItem) => void;
}) {
  const { mr, repoName, unresolved } = item;
  const [ci, setCi] = useState<{ status: string; url: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<{ status: string; url: string } | null>('get_mr_ci', { mrId: mr.id })
      .then((r) => { if (!cancelled) setCi(r ?? null); })
      .catch(() => { if (!cancelled) setCi(null); });
    return () => { cancelled = true; };
  }, [mr.id]);

  const shortKind = mr.platform === 'github' ? '#' : '!';

  return (
    <button
      className="forge-mr-row"
      onClick={() => onSelect(item)}
      title={`${mr.url} — open overview`}
    >
      <span className={`forge-state forge-state-${mr.state}`}>
        <StateIcon state={mr.state} />
      </span>
      <span className="forge-mr-num">{shortKind}{mr.remote_id}</span>
      {multiRepo && <span className="forge-mr-repo">{repoName}</span>}
      <span className="forge-mr-right">
        {ci && (
          <span className={`forge-ci forge-ci-${ciGroup(ci.status)}`} title={`Pipeline: ${ci.status}`}>
            <span className="forge-ci-dot" />
          </span>
        )}
        {unresolved > 0 && (
          <span className="forge-mr-unresolved" title={`${unresolved} unresolved thread${unresolved > 1 ? 's' : ''}`}>
            <MessageSquare size={11} strokeWidth={1.75} />
            {unresolved}
          </span>
        )}
      </span>
    </button>
  );
}

/** Compact, selectable list of the task's MRs — selecting one opens its
 *  overview tab (threads, description, CI details live there). */
export function ForgeSection({
  items, onSelect,
}: {
  items: ForgeItem[];
  onSelect: (item: ForgeItem) => void;
}) {
  if (items.length === 0) return <div className="sidebar-empty">No merge request</div>;
  const multiRepo = new Set(items.map((i) => i.repoId)).size > 1;
  return (
    <div className="forge">
      {items.map((item) => (
        <ForgeRow key={item.mr.id} item={item} multiRepo={multiRepo} onSelect={onSelect} />
      ))}
    </div>
  );
}
