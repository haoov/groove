import { useCallback, useEffect, useState } from 'react';
import { call } from '../shared/ipc/client';
import { applyUiConfig } from '../shared/lib/ui';
import { AuthTerminal } from './AuthTerminal';
import type { ConfigView, Environment, DetectedSchema, NotionUser } from '../shared/ipc/generated';

/** First-run setup: check the environment, then take the Notion token, the task
 *  database, an identity, and the worktree root, and write the config. Order and
 *  arg names mirror the backend (check_environment → detect_database →
 *  write_initial_config). */
export function FirstRun({ onReady }: { onReady: (cfg: ConfigView) => void }) {
  const [env, setEnv] = useState<Environment | null>(null);
  const [authSid, setAuthSid] = useState<string | null>(null);

  const [token, setToken] = useState('');
  const [databaseId, setDatabaseId] = useState('');
  const [email, setEmail] = useState('');
  const [worktreeRoot, setWorktreeRoot] = useState('~/worktrees');
  const [templatePageId, setTemplatePageId] = useState('');

  const [schema, setSchema] = useState<DetectedSchema | null>(null);
  const [user, setUser] = useState<NotionUser | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const checkEnv = useCallback(() => {
    call<Environment>('check_environment').then(setEnv).catch((e) => setError(String(e)));
  }, []);
  useEffect(() => { checkEnv(); }, [checkEnv]);

  const missingRequired = (env?.tools ?? []).filter((t) => t.required && !t.path);
  const blocked = missingRequired.length > 0;

  const detect = async () => {
    setSchema(null);
    if (!token.trim() || !databaseId.trim()) return;
    try { setSchema(await call<DetectedSchema>('detect_database', { token: token.trim(), databaseId: databaseId.trim() })); setError(null); }
    catch (e) { setError(String(e)); }
  };

  const lookupUser = async () => {
    setUser(null);
    if (!token.trim() || !email.trim()) return;
    try { setUser(await call<NotionUser>('find_notion_user', { token: token.trim(), email: email.trim() })); setError(null); }
    catch (e) { setError(String(e)); }
  };

  const signIn = async () => {
    try { setAuthSid(await call<string>('start_auth_session')); } catch (e) { setError(String(e)); }
  };

  const canSave = !blocked && token.trim() && databaseId.trim() && worktreeRoot.trim() && !!schema;

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await call('write_initial_config', {
        token: token.trim(),
        databaseId: databaseId.trim(),
        userId: user?.id ?? '',
        worktreeRoot: worktreeRoot.trim(),
        templatePageId: templatePageId.trim() || null,
      });
      const cfg = await call<ConfigView | null>('get_config');
      if (!cfg) throw new Error('config did not save');
      applyUiConfig(cfg.ui);
      onReady(cfg);
    } catch (e) { setError(String(e)); setSaving(false); }
  };

  return (
    <div className="firstrun">
      <div className="fr-card">
        <h1>Set up Groove</h1>
        <p className="fr-sub">Connect your Notion task database and pick where worktrees live.</p>

        <section className="fr-sec">
          <h2>Environment</h2>
          <ul className="fr-tools">
            {env?.tools.map((t) => (
              <li key={t.name} className={!t.path ? (t.required ? 'bad' : 'warn') : t.authed === false ? 'warn' : 'ok'}>
                <span className="fr-tool-name">{t.name}</span>
                <span className="fr-tool-purpose">{t.purpose}</span>
                <span className="fr-tool-state">
                  {!t.path ? (t.required ? 'missing (required)' : 'missing') : t.authed === false ? 'not signed in' : 'ok'}
                </span>
                {t.path && t.authed === false && <button className="fr-mini" onClick={signIn}>Sign in</button>}
              </li>
            ))}
          </ul>
          {authSid && (
            <div className="fr-auth">
              <AuthTerminal sessionId={authSid} />
              <button className="fr-mini" onClick={() => { setAuthSid(null); checkEnv(); }}>Done — recheck</button>
            </div>
          )}
        </section>

        <section className="fr-sec">
          <h2>Notion</h2>
          <label className="fr-field"><span>Integration token</span>
            <input type="password" value={token} onChange={(e) => setToken(e.target.value)} onBlur={detect} placeholder="ntn_…" />
          </label>
          <label className="fr-field"><span>Task database id</span>
            <input value={databaseId} onChange={(e) => setDatabaseId(e.target.value)} onBlur={detect} placeholder="32-char id or share URL" />
          </label>
          {schema && (
            <div className="fr-detected">
              <b>Detected:</b> title “{schema.title_property}”, status “{schema.status_property}” ·
              {' '}{schema.status_ready} → {schema.status_in_progress} → {schema.status_done}
              {' '}({schema.status_options.length} states)
            </div>
          )}
          <label className="fr-field"><span>Your email (assignee filter — optional)</span>
            <input value={email} onChange={(e) => setEmail(e.target.value)} onBlur={lookupUser} placeholder="you@company.com" />
          </label>
          {user && <div className="fr-detected">You: {user.name}{user.email ? ` · ${user.email}` : ''}</div>}
        </section>

        <section className="fr-sec">
          <h2>Worktrees</h2>
          <label className="fr-field"><span>Worktree root</span>
            <input value={worktreeRoot} onChange={(e) => setWorktreeRoot(e.target.value)} />
          </label>
          <label className="fr-field"><span>Task template page id (optional)</span>
            <input value={templatePageId} onChange={(e) => setTemplatePageId(e.target.value)} placeholder="blank for none" />
          </label>
        </section>

        {error && <div className="fr-error">{error}</div>}
        <div className="fr-actions">
          <button className="fr-save" disabled={!canSave || saving} onClick={save}>{saving ? 'Saving…' : 'Save and start'}</button>
        </div>
      </div>
    </div>
  );
}
