import type { Hunk } from '../shared/ipc/generated';

/** Render a file's hunks. Git's own context already collapses the gaps between
 *  hunks; each hunk header marks the jump. */
export function DiffLines({ hunks }: { hunks: Hunk[] }) {
  if (hunks.length === 0) return <div className="diff-empty">No line changes.</div>;
  return (
    <div className="diff">
      {hunks.map((h, hi) => (
        <div key={hi} className="hunk">
          <div className="dl h">{h.header}</div>
          {h.lines.map((l, li) => (
            <div key={li} className={`dl ${l.type}`}>
              <span className="ln">{l.type === 'del' ? '' : l.num || ''}</span>
              <span className="sign">{l.type === 'add' ? '+' : l.type === 'del' ? '-' : ' '}</span>
              <span className="tx">{l.content}</span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
