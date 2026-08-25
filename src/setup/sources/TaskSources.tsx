import { useState } from 'react';
import { Check, Loader2, Minus } from 'lucide-react';
import { invoke } from '../../shared/ipc/invoke';
import type { Config, Environment } from '../../shared/ipc/ipc';

/** Which sources are on, what they point at, and how to fix a source that has
 *  stopped working. Not a second setup screen — the same shape as "This machine". */
export function TaskSources({
  config, env, onChanged, onNeedsScope,
}: {
  config: Config | null;
  env: Environment | null;
  onChanged: () => void;
  onNeedsScope: () => void;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const gh = env?.tools.find((t) => t.name === 'gh');
  // Unknown scopes are not missing scopes: a GH_TOKEN prints none.
  const missingScope = !!gh?.path && gh.authed === true && !!gh.scopes && !gh.scopes.includes('project');

  const toggleGithub = async (enabled: boolean) => {
    setBusy('github');
    setError(null);
    try {
      await invoke('set_task_source', { provider: 'github', enabled, options: null });
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const removeNotion = async () => {
    setBusy('notion');
    setError(null);
    try {
      await invoke('set_task_source', { provider: 'notion', enabled: false, options: null });
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <ul className="firstrun-tools">
        <li className={config?.notion ? 'ok' : 'optional'}>
          {config?.notion ? <Check size={12} strokeWidth={2.5} /> : <Minus size={12} strokeWidth={2} />}
          <code>notion</code>
          <span className="firstrun-tool-purpose">
            {config?.notion
              ? `database ${config.notion.database_id.slice(-8)}`
              : 'not set up — add it in the config file'}
          </span>
          {config?.notion && (
            <button className="firstrun-signin" disabled={busy !== null} onClick={removeNotion}>
              {busy === 'notion' ? <Loader2 size={11} className="spin" /> : 'disconnect'}
            </button>
          )}
        </li>

        <li className={config?.github ? (missingScope ? 'optional' : 'ok') : 'optional'}>
          {config?.github && !missingScope
            ? <Check size={12} strokeWidth={2.5} />
            : <Minus size={12} strokeWidth={2} />}
          <code>github</code>
          <span className="firstrun-tool-purpose">
            {!config?.github
              ? 'not set up — issues assigned to you that sit on a board'
              : missingScope
                ? 'missing the project scope — tasks are read-only'
                : 'issues assigned to you that sit on a board'}
          </span>
          {config?.github && missingScope ? (
            <button className="firstrun-signin" onClick={onNeedsScope}>grant access</button>
          ) : (
            <button
              className="firstrun-signin"
              disabled={busy !== null}
              onClick={() => toggleGithub(!config?.github)}
            >
              {busy === 'github'
                ? <Loader2 size={11} className="spin" />
                : config?.github ? 'disconnect' : 'connect'}
            </button>
          )}
        </li>
      </ul>

      {error && <p className="settings-hint settings-error">{error}</p>}
      <p className="settings-hint">
        Tasks from every connected source share one queue. Removing the last one is refused —
        the app has nothing to show without it.
      </p>
    </>
  );
}
