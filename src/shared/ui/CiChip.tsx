import { useState } from 'react';
import { ChevronDown, ExternalLink, Wrench } from 'lucide-react';
import { ContextMenu } from './ContextMenu';
import { ciGroup } from '../lib/mr';
import { forgeName } from '../lib/forge';
import { openExternal } from '../lib/openExternal';
import { sendSkill } from '../lib/agentSend';
import { offers } from '../lib/skills';
import { useStore, useSession } from '../store';

/**
 * The MR's pipeline chip, and what you can do about it.
 *
 * Both surfaces that show CI use this one — the commit panel's chip and the MR
 * overview's badge differ only in `className` and label, and a menu written
 * twice drifts.
 */
export function CiChip({ status, url, platform, className, children }: {
  status: string;
  /** The run's URL; the MR's own page when the forge reported none. */
  url: string;
  platform: string;
  className: string;
  children: React.ReactNode;
}) {
  const sessionId = useSession((s) => s.id);
  const setLastError = useStore((s) => s.setLastError);
  const kind = useSession((s) => s.kind);
  // The same question the Actions menu asks: fix-ci is task-only, and this chip
  // renders in a review session too.
  const hasFixCi = useStore((s) => offers(s.skills, kind, 'groove:fix-ci'));
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);

  const failed = ciGroup(status) === 'fail';

  const fixCi = async () => {
    setAt(null);
    useStore.getState().requestConsoleFocus();
    try {
      await sendSkill(sessionId, 'groove:fix-ci');
    } catch (e) {
      setLastError(String(e));
    }
  };

  return (
    <>
      <button
        className={`${className} forge-ci-${ciGroup(status)}`}
        onClick={(e) => setAt({ x: e.clientX, y: e.clientY })}
        title={`Pipeline: ${status.replace(/_/g, ' ')}`}
      >
        {children}
        <ChevronDown size={11} strokeWidth={2} className="ci-caret" />
      </button>
      {at && (
        <ContextMenu x={at.x} y={at.y} onClose={() => setAt(null)}>
          <button className="ctx-menu-item" onClick={() => { setAt(null); openExternal(url); }}>
            <ExternalLink size={13} strokeWidth={1.75} />
            Open on {forgeName(platform)}
          </button>
          {/* Only for a red pipeline: there is nothing to fix on a green one, and
              the skill is gone if the user deleted it from the core set. */}
          {failed && hasFixCi && (
            <button className="ctx-menu-item" onClick={() => void fixCi()}>
              <Wrench size={13} strokeWidth={1.75} />
              Fix CI
            </button>
          )}
        </ContextMenu>
      )}
    </>
  );
}
