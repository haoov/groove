import { useEffect, useState } from 'react';
import { invoke } from '../../shared/ipc/invoke';
import type { GithubPreview } from '../../shared/ipc/ipc';

/** Nothing to configure: a task is an open issue assigned to you that somebody has
 *  put on a board. The form is just a preview of what that comes to. */
export function GithubSetup({ onNeedsScope }: { onNeedsScope: () => void }) {
  const [preview, setPreview] = useState<GithubPreview | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<GithubPreview>('preview_github', {})
      .then((p) => { setPreview(p); setError(null); })
      .catch((e) => {
        const msg = String(e);
        if (/required scopes|not logged|auth/i.test(msg)) onNeedsScope();
        setError(msg);
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <>
      <p className="firstrun-note">
        Open issues assigned to you that sit on a project board become tasks. An issue on
        no board is not a task — that is the filter, and there is nothing to pick.
      </p>

      {error && <div className="firstrun-warn"><span>{error}</span></div>}

      {preview && (
        <div className="firstrun-detected">
          <div className="firstrun-detected-head">What Groove can see</div>
          <dl className="firstrun-detected-list">
            <dt>Tasks</dt><dd>{preview.tasks}</dd>
            <dt>Boards</dt>
            <dd>{preview.boards.length ? preview.boards.join(' · ') : <em>none</em>}</dd>
            <dt>Fields</dt>
            <dd>{preview.fields.length ? preview.fields.join(' · ') : <em>none</em>}</dd>
          </dl>
          <span className="firstrun-hint">
            {preview.unboarded > 0 && (
              <>{preview.unboarded} assigned {preview.unboarded === 1 ? 'issue is' : 'issues are'} on
              no board and will not appear. </>
            )}
            Status and Priority come from each board's own fields.
          </span>
        </div>
      )}
    </>
  );
}
