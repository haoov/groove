import { useEffect, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { AlertTriangle, Check, Loader2, RefreshCw } from 'lucide-react';
import { useStore } from '../shared/store';
import { AuthModal } from './AuthModal';
import { SOURCES, SOURCE_IDS } from './sources';
import { applyFontFamily, applyFontSize, applyTheme } from '../shared/lib/theme';
import { forgeCliState } from '../shared/lib/forge';
import {
  DEFAULT_FONT_SIZE, DEFAULT_THEME, type Config, type Environment, type ProviderId,
} from '../shared/ipc/ipc';

/**
 * What a new machine sees: the dependency list, and the four values only the user
 * can supply.
 *
 * Without this the first run was a Home page with no tasks and a Notion error in
 * the corner — nothing to say WHAT was missing. Everything asked for here is
 * something the app cannot discover: a token, which database, which person, and
 * where to put worktrees. The rest of the config is written with defaults and can
 * be edited in the file, whose path is printed below.
 */

export function FirstRun({ onReady }: { onReady: (cfg: Config) => void }) {
  const setLastError = useStore((s) => s.setLastError);
  const [env, setEnv] = useState<Environment | null>(null);
  const [root, setRoot] = useState('~/worktrees');
  // Per source: is it on, and the payload its form reports (null = incomplete).
  const [on, setOn] = useState<Partial<Record<ProviderId, boolean>>>({});
  const [payloads, setPayloads] = useState<Partial<Record<ProviderId, unknown>>>({});
  /** Which forge CLI is being signed in, if any. */
  const [authing, setAuthing] = useState<{ tool: 'glab' | 'gh'; mode: 'login' | 'scope' } | null>(null);
  const [busy, setBusy] = useState<'save' | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadEnv = () => {
    invoke<Environment>('check_environment').then(setEnv).catch((e) => setError(String(e)));
  };
  useEffect(loadEnv, []);

  const save = async () => {
    setBusy('save');
    setError(null);
    try {
      // SetupRequest's field names ARE the ProviderId strings, so the enabled
      // payloads splice straight in.
      const sources = Object.fromEntries(
        SOURCE_IDS.filter((id) => on[id]).map((id) => [id, payloads[id] ?? null]),
      );
      await invoke('write_initial_config', {
        setup: { worktree_root: root.trim(), ...sources },
      });
      const cfg = await invoke<Config | null>('get_config');
      if (!cfg) throw new Error('Config saved but not readable — check the path below.');
      applyFontSize(cfg.ui?.font_size ?? DEFAULT_FONT_SIZE);
      applyFontFamily(cfg.ui?.font_family);
      applyTheme(cfg.ui?.theme ?? DEFAULT_THEME);
      onReady(cfg);
    } catch (e) {
      setError(String(e));
      setLastError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const missingRequired = (env?.tools ?? []).filter((t) => t.required && !t.path);

  // An enabled-but-incomplete source blocks saving; at least one must be on. A
  // form with nothing to fill in reports its payload on mount.
  const enabled = SOURCE_IDS.filter((id) => on[id]);
  const canSave =
    enabled.length > 0 &&
    enabled.every((id) => payloads[id] != null) &&
    !!root.trim() &&
    busy === null;

  return (
    <div className="firstrun">
      {authing && (
        <AuthModal
          tool={authing.tool}
          mode={authing.mode}
          onDone={() => { setAuthing(null); loadEnv(); }}
        />
      )}
      <div className="firstrun-card">
        <h1 className="firstrun-title">Set up Groove</h1>
        <p className="firstrun-lead">
          Groove pulls tasks from the sources you connect and drives a pool of git
          worktrees. It only asks for what it cannot work out on its own.
        </p>

        {env?.config_error && (
          <div className="firstrun-warn">
            <AlertTriangle size={13} strokeWidth={2} />
            <span>
              A config exists but could not be read: {env.config_error}. Fix or delete{' '}
              <code>{env.config_path}</code> — saving below overwrites it.
            </span>
          </div>
        )}

        <section className="firstrun-section">
          <h2 className="firstrun-h2">
            On this machine
            <button className="home-link" onClick={loadEnv} title="Check again">
              <RefreshCw size={11} strokeWidth={2.2} />
              recheck
            </button>
          </h2>
          {!env ? (
            <p className="firstrun-note">Checking…</p>
          ) : (
            <ul className="firstrun-tools">
              {env.tools.map((t) => {
                // Installed but not logged in is its own state: the tool is there,
                // and every MR feature still fails until the CLI has credentials.
                // One rule, shared with the settings view — it also knows why an
                // unreadable scopes list is not a missing one.
                const state = forgeCliState(t);
                const needsAuth = state === 'needs-auth';
                const needsScope = state === 'needs-scope';
                const cls = !t.path
                  ? (t.required ? 'missing' : 'optional')
                  : needsAuth || needsScope ? 'optional' : 'ok';
                return (
                  <li key={t.name} className={cls}>
                    {t.path && !needsAuth && !needsScope
                      ? <Check size={12} strokeWidth={2.5} />
                      : <AlertTriangle size={12} strokeWidth={2} />}
                    <code>{t.name}</code>
                    <span className="firstrun-tool-purpose">
                      {needsAuth
                        ? 'installed, but not signed in'
                        : needsScope
                          ? 'signed in, but missing the project scope — GitHub tasks stay read-only'
                          // The tick proves a file on $PATH, nothing more — say so.
                          : t.path ? `installed — ${t.purpose}` : t.purpose}
                    </span>
                    {(needsAuth || needsScope) && (t.name === 'glab' || t.name === 'gh') ? (
                      <button
                        className="firstrun-signin"
                        onClick={() => setAuthing({
                          tool: t.name as 'glab' | 'gh',
                          mode: needsScope ? 'scope' : 'login',
                        })}
                        title={needsScope ? 'Widen the token here' : `Run ${t.name} auth login here`}
                      >
                        {needsScope ? 'grant access' : 'sign in'}
                      </button>
                    ) : (
                      !t.path && <span className="firstrun-tool-tag">{t.required ? 'required' : 'optional'}</span>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
          {missingRequired.length > 0 && (
            <p className="firstrun-note">
              Install {missingRequired.map((t) => t.name).join(' and ')} before going further —
              nothing works without {missingRequired.length > 1 ? 'them' : 'it'}.
            </p>
          )}
        </section>

        {SOURCE_IDS.map((id) => {
          const src = SOURCES[id];
          const Form = src.SetupForm;
          return (
            <section key={id} className="firstrun-section">
              <h2 className="firstrun-h2">
                <label className="firstrun-source-toggle">
                  <input
                    type="checkbox"
                    checked={!!on[id]}
                    onChange={(e) => setOn((prev) => ({ ...prev, [id]: e.target.checked }))}
                  />
                  {src.label}
                </label>
              </h2>
              {on[id] && (
                <Form
                  onChange={(payload) => setPayloads((prev) => ({ ...prev, [id]: payload }))}
                  onNeedsScope={loadEnv}
                />
              )}
            </section>
          );
        })}

        <section className="firstrun-section">
          <h2 className="firstrun-h2">Worktrees</h2>
          <label className="firstrun-field">
            <span className="firstrun-label">Worktree root</span>
            <input
              className="firstrun-input"
              placeholder="~/worktrees"
              value={root}
              onChange={(e) => setRoot(e.target.value)}
            />
            <span className="firstrun-hint">
              Created if missing. Clones live in <code>&lt;root&gt;/main/&lt;host&gt;/&lt;group&gt;/&lt;repo&gt;</code> and
              each task gets <code>&lt;root&gt;/worktrees/&lt;TASK-ID&gt;/&lt;repo&gt;</code>. Point it at an empty
              directory unless you already use this layout.
            </span>
          </label>
        </section>

        {error && (
          <div className="firstrun-error">
            <AlertTriangle size={13} strokeWidth={2} />
            <span>{error}</span>
          </div>
        )}

        <div className="firstrun-actions">
          <span className="firstrun-path">Saved to {env?.config_path ?? '…'}</span>
          <button className="btn-primary" disabled={!canSave} onClick={save}>
            {busy === 'save' ? <Loader2 size={11} className="spin" /> : null}
            Save and start
          </button>
        </div>
      </div>
    </div>
  );
}
