import { useEffect, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useSession } from '../shared/store';
import type { CiStatus, Mr } from '../shared/ipc/ipc';
import { CiChip } from '../shared/ui/CiChip';

/** The MR's pipeline status; grey when the forge reports none. */
export function MrCiChip({ mr }: { mr: Mr }) {
  const [ci, setCi] = useState<CiStatus | null>(null);
  // The only trigger: a push, an mr.* op, or the sidebar refresh. Nothing polls the forge.
  const mrNonce = useSession((s) => s.mrNonce);

  useEffect(() => {
    let cancelled = false;
    invoke<CiStatus | null>('get_mr_ci', { mrId: mr.id })
      .then((r) => { if (!cancelled) setCi(r ?? null); })
      .catch(() => { if (!cancelled) setCi(null); });
    return () => { cancelled = true; };
  }, [mr.id, mrNonce]);

  if (!ci) {
    return (
      <span className="git-commit-mr-ci forge-ci-idle" title="No pipeline reported">
        <span className="forge-ci-dot" />
        CI
      </span>
    );
  }
  return (
    <CiChip status={ci.status} url={ci.url || mr.url} platform={mr.platform} className="git-commit-mr-ci">
      <span className="forge-ci-dot" />
      CI
    </CiChip>
  );
}
