import { Check, AlertTriangle, Minus } from 'lucide-react';
import { FORGE_CLIS, forgeCliState } from '../shared/lib/forge';
import type { Config, Environment } from '../shared/ipc/ipc';

/** Row text for a CLI that is not ready. */
const PURPOSE = {
  missing: 'not installed',
  'needs-auth': 'installed, but not signed in',
  'needs-scope': 'signed in, but missing the project scope — GitHub tasks stay read-only',
} as const;

export function GitForgePanel({
  config, env, onSignIn,
}: {
  config: Config | null;
  env: Environment | null;
  onSignIn: (tool: 'glab' | 'gh', mode: 'login' | 'scope') => void;
}) {
  const clis = (env?.tools ?? []).filter((t) => FORGE_CLIS.includes(t.name));

  return (
    <>
      <section className="settings-section">
        <div className="settings-section-title">Worktree root</div>
        <p className="settings-value">
          <code>{config?.git?.worktree_root || '—'}</code>
        </p>
        <p className="settings-hint">
          Where clones and worktrees live: repos under <code>main/</code>, sessions under
          <code> worktrees/</code>. Set at first run and not editable here — it names
          directories that already exist.
        </p>
      </section>

      <section className="settings-section">
        <div className="settings-section-title">Forge sign-in</div>
        {!env ? (
          <p className="settings-hint">Checking…</p>
        ) : (
          <ul className="firstrun-tools">
            {clis.map((t) => {
              const state = forgeCliState(t);
              return (
                <li key={t.name} className={state === 'ready' ? 'ok' : 'optional'}>
                  {state === 'ready'
                    ? <Check size={12} strokeWidth={2.5} />
                    : state === 'missing'
                      ? <Minus size={12} strokeWidth={2} />
                      : <AlertTriangle size={12} strokeWidth={2} />}
                  <code>{t.name}</code>
                  <span className="firstrun-tool-purpose">
                    {state === 'ready' ? `signed in — ${t.purpose}` : PURPOSE[state]}
                  </span>
                  {state === 'needs-auth' || state === 'needs-scope' ? (
                    <button
                      className="firstrun-signin"
                      onClick={() => onSignIn(t.name as 'glab' | 'gh', state === 'needs-scope' ? 'scope' : 'login')}
                      title={state === 'needs-scope' ? 'Widen the token here' : `Run ${t.name} auth login here`}
                    >
                      {state === 'needs-scope' ? 'grant access' : 'sign in'}
                    </button>
                  ) : (
                    state === 'missing' && <span className="firstrun-tool-tag">optional</span>
                  )}
                </li>
              );
            })}
          </ul>
        )}
        <p className="settings-hint">
          One CLI per forge, each holding its own credentials. Only the forges you have
          repos on matter — MRs, threads and CI status come from these.
        </p>
      </section>
    </>
  );
}
