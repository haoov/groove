import { useEffect, useState } from 'react';
import { Check, Loader2, Minus } from 'lucide-react';
import { invoke } from '../../shared/ipc/invoke';
import type { GithubPreview } from '../../shared/ipc/ipc';
import type { SetupFormProps, SettingsRowProps } from './index';

/** Nothing to configure: a task is an open issue assigned to you that somebody has
 *  put on a board. The form is just a preview of what that comes to. */
export function GithubSetupForm({ onChange, onNeedsScope }: SetupFormProps) {
  const [preview, setPreview] = useState<GithubPreview | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Enabling is enough — gh already holds the credential.
  useEffect(() => {
    onChange({ host: null });
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
            {/* A board with no Ready/In progress/Done-like column makes Finish
                fail; showing the real names is what makes that fixable. */}
            {preview.status_columns.map((b) => (
              <span key={b.board} style={{ display: 'contents' }}>
                <dt>{b.board} states</dt>
                <dd>{b.columns.length ? b.columns.join(' · ') : <em>no Status field</em>}</dd>
              </span>
            ))}
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

/** Settings: connect/disconnect, plus the gh scope fix when tasks are read-only. */
export function GithubSettingsRow({ config, env, busy, setSource, onNeedsScope }: SettingsRowProps) {
  const gh = env?.tools.find((t) => t.name === 'gh');
  // Unknown scopes are not missing scopes: a GH_TOKEN prints none.
  const missingScope = !!gh?.path && gh.authed === true && !!gh.scopes && !gh.scopes.includes('project');
  const on = !!config?.github;

  return (
    <li className={on ? (missingScope ? 'optional' : 'ok') : 'optional'}>
      {on && !missingScope
        ? <Check size={12} strokeWidth={2.5} />
        : <Minus size={12} strokeWidth={2} />}
      <code>github</code>
      <span className="firstrun-tool-purpose">
        {!on
          ? 'not set up — issues assigned to you that sit on a board'
          : missingScope
            ? 'missing the project scope — tasks are read-only'
            : 'issues assigned to you that sit on a board'}
      </span>
      {on && missingScope ? (
        <button className="firstrun-signin" onClick={onNeedsScope}>grant access</button>
      ) : (
        <button
          className="firstrun-signin"
          disabled={busy}
          onClick={() => setSource(!on, null)}
        >
          {busy ? <Loader2 size={11} className="spin" /> : on ? 'disconnect' : 'connect'}
        </button>
      )}
    </li>
  );
}
