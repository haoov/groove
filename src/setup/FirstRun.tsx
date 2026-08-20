import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertTriangle, Check, Loader2, RefreshCw } from 'lucide-react';
import { useStore } from '../shared/store';
import { AuthModal } from './AuthModal';
import { applyFontFamily, applyFontSize, applyTheme } from '../shared/lib/theme';
import { DEFAULT_FONT_SIZE, type Config } from '../shared/ipc/ipc';
import { resolveUser, userLabel, type NotionUser } from '../shared/lib/notionUser';

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

export interface ToolCheck {
  name: string;
  path: string | null;
  purpose: string;
  required: boolean;
  /** For the forge CLIs: logged in or not. Null when absent or not applicable. */
  authed: boolean | null;
}

/** Shared with Settings, which shows the same check after setup. */
export interface Environment {
  config_path: string;
  config_exists: boolean;
  config_error: string | null;
  tools: ToolCheck[];
}

/** What the database says about itself (mirror of setup.rs::DetectedSchema). */
interface DetectedSchema {
  title_property: string;
  status_property: string;
  priority_property: string | null;
  sprint_property: string | null;
  project_property: string | null;
  assignee_property: string | null;
  status_ready: string;
  status_in_progress: string;
  status_done: string;
  status_options: string[];
}

export function FirstRun({ onReady }: { onReady: (cfg: Config) => void }) {
  const setLastError = useStore((s) => s.setLastError);
  const [env, setEnv] = useState<Environment | null>(null);
  const [token, setToken] = useState('');
  const [databaseId, setDatabaseId] = useState('');
  const [root, setRoot] = useState('~/worktrees');
  const [users, setUsers] = useState<NotionUser[] | null>(null);
  const [who, setWho] = useState('');
  const [template, setTemplate] = useState('');
  const [detected, setDetected] = useState<DetectedSchema | null>(null);
  /** Which forge CLI is being signed in, if any. */
  const [authing, setAuthing] = useState<'glab' | 'gh' | null>(null);
  const [busy, setBusy] = useState<'users' | 'detect' | 'save' | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadEnv = () => {
    invoke<Environment>('check_environment').then(setEnv).catch((e) => setError(String(e)));
  };
  useEffect(loadEnv, []);

  // The token is checked by using it: the people list is both the check and the
  // way to pick yourself without hunting a UUID in the API.
  const loadUsers = async () => {
    if (!token.trim()) return;
    setBusy('users');
    setError(null);
    try {
      const list = await invoke<NotionUser[]>('list_notion_users', { token: token.trim() });
      setUsers(list);
      if (list.length === 0) {
        // The integration needs the "read user information" capability; without it
        // the call succeeds and returns nobody, which reads as a broken picker.
        setError('The token works, but no people came back — the integration needs the user-information capability.');
      }
    } catch (e) {
      setUsers(null);
      setError(`Notion rejected the token: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  // Reading the schema replaces asking for eleven property names. It also proves
  // the integration can see the database, so it doubles as the id check.
  const detect = async () => {
    if (!token.trim() || !databaseId.trim()) return;
    setBusy('detect');
    setError(null);
    try {
      setDetected(await invoke<DetectedSchema>('detect_database', {
        token: token.trim(),
        databaseId: databaseId.trim(),
      }));
    } catch (e) {
      setDetected(null);
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const save = async () => {
    setBusy('save');
    setError(null);
    try {
      await invoke('write_initial_config', {
        token: token.trim(),
        databaseId: databaseId.trim(),
        userId: whoMatch.kind === 'user' || whoMatch.kind === 'raw' ? whoMatch.id : '',
        worktreeRoot: root.trim(),
        templatePageId: template.trim() || null,
      });
      const cfg = await invoke<Config | null>('get_config');
      if (!cfg) throw new Error('Config saved but not readable — check the path below.');
      applyFontSize(cfg.ui?.font_size ?? DEFAULT_FONT_SIZE);
      applyFontFamily(cfg.ui?.font_family);
      applyTheme(cfg.ui?.theme ?? 'frappe');
      onReady(cfg);
    } catch (e) {
      setError(String(e));
      setLastError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const whoMatch = resolveUser(who, users ?? []);
  const missingRequired = (env?.tools ?? []).filter((t) => t.required && !t.path);
  const canSave = !!token.trim() && !!databaseId.trim() && !!root.trim() && busy === null;

  return (
    <div className="firstrun">
      {authing && (
        <AuthModal
          tool={authing}
          onDone={() => { setAuthing(null); loadEnv(); }}
        />
      )}
      <div className="firstrun-card">
        <h1 className="firstrun-title">Set up Groove</h1>
        <p className="firstrun-lead">
          Groove drives your Notion task database and a pool of git worktrees. It needs four
          things it cannot work out on its own.
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
                const needsAuth = !!t.path && t.authed === false;
                const cls = !t.path ? (t.required ? 'missing' : 'optional') : needsAuth ? 'optional' : 'ok';
                return (
                  <li key={t.name} className={cls}>
                    {t.path && !needsAuth
                      ? <Check size={12} strokeWidth={2.5} />
                      : <AlertTriangle size={12} strokeWidth={2} />}
                    <code>{t.name}</code>
                    <span className="firstrun-tool-purpose">
                      {needsAuth ? 'installed, but not signed in' : t.purpose}
                    </span>
                    {needsAuth && (t.name === 'glab' || t.name === 'gh') ? (
                      <button
                        className="firstrun-signin"
                        onClick={() => setAuthing(t.name as 'glab' | 'gh')}
                        title={`Run ${t.name} auth login here`}
                      >
                        sign in
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

        <section className="firstrun-section">
          <h2 className="firstrun-h2">Notion</h2>
          <label className="firstrun-field">
            <span className="firstrun-label">Integration token</span>
            <input
              className="firstrun-input"
              type="password"
              autoFocus
              placeholder="ntn_…"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              onBlur={loadUsers}
            />
            <span className="firstrun-hint">
              notion.so/my-integrations → your integration → Internal Integration Secret. The task
              database must be shared with that integration.
            </span>
          </label>

          <label className="firstrun-field">
            <span className="firstrun-label">Task database id</span>
            <div className="firstrun-row">
              <input
                className="firstrun-input"
                placeholder="32 hex characters, from the database URL"
                value={databaseId}
                onChange={(e) => { setDatabaseId(e.target.value); setDetected(null); }}
                onBlur={detect}
              />
              <button
                className="btn-secondary"
                onClick={detect}
                disabled={!token.trim() || !databaseId.trim() || busy !== null}
              >
                {busy === 'detect' ? <Loader2 size={11} className="spin" /> : null}
                read schema
              </button>
            </div>
            <span className="firstrun-hint">
              Open the database as a full page: the id is the last path segment before <code>?v=</code>.
            </span>
          </label>

          {detected && (
            <div className="firstrun-detected">
              <div className="firstrun-detected-head">Read from the database</div>
              <dl className="firstrun-detected-list">
                <dt>Title</dt><dd>{detected.title_property}</dd>
                <dt>Status</dt><dd>{detected.status_property}</dd>
                <dt>Priority</dt><dd>{detected.priority_property ?? <em>none</em>}</dd>
                <dt>Sprint</dt><dd>{detected.sprint_property ?? <em>none</em>}</dd>
                <dt>Project</dt><dd>{detected.project_property ?? <em>none</em>}</dd>
                <dt>Assignee</dt><dd>{detected.assignee_property ?? <em>none</em>}</dd>
              </dl>
              <div className="firstrun-detected-head">Status values Groove will set</div>
              <dl className="firstrun-detected-list">
                <dt>Filing a task</dt><dd>{detected.status_ready || <em>not found</em>}</dd>
                <dt>Picking it up</dt><dd>{detected.status_in_progress || <em>not found</em>}</dd>
                <dt>Finishing it</dt><dd>{detected.status_done || <em>not found</em>}</dd>
              </dl>
              <span className="firstrun-hint">
                Detected from the property types and Notion's own status groups. Every value is
                written to the config file and can be corrected there.
                {detected.status_options.length > 0 && (
                  <> All options: {detected.status_options.join(' · ')}.</>
                )}
              </span>
            </div>
          )}

          <label className="firstrun-field">
            <span className="firstrun-label">Task template page id <span className="firstrun-optional">optional</span></span>
            <input
              className="firstrun-input"
              placeholder="Page whose body seeds a new task"
              value={template}
              onChange={(e) => setTemplate(e.target.value)}
            />
            <span className="firstrun-hint">
              Required to turn an explorer session into a task, and used as the body of a
              hand-filed one. Open the database's template as a full page and copy its link —
              the id is the last path segment. Checked when you save.
            </span>
          </label>

          <label className="firstrun-field">
            <span className="firstrun-label">You, in Notion <span className="firstrun-optional">optional</span></span>
            <div className="firstrun-row">
              {/* A filterable input, not a dropdown: a workspace has hundreds of
                  people, and a pasted id still has to work when the list fails. */}
              <input
                className="firstrun-input"
                list="groove-notion-users"
                placeholder={users ? 'Type your name, or paste a user id' : 'Enter the token first'}
                value={who}
                onChange={(e) => setWho(e.target.value)}
              />
              <datalist id="groove-notion-users">
                {(users ?? []).map((u) => <option key={u.id} value={userLabel(u)} />)}
              </datalist>
              <button className="btn-secondary" onClick={loadUsers} disabled={!token.trim() || busy !== null}>
                {busy === 'users' ? <Loader2 size={11} className="spin" /> : null}
                {users ? 'reload' : 'check token'}
              </button>
            </div>
            <span className="firstrun-hint">
              {whoMatch.kind === 'user' && (
                <>Matched <strong>{whoMatch.user.name}</strong> · <code>{whoMatch.id}</code>. </>
              )}
              {whoMatch.kind === 'raw' && (
                <>Using <code>{whoMatch.id}</code> as-is — it is not in this workspace's people, so
                  check it is a USER id and not a page id. </>
              )}
              {whoMatch.kind === 'unknown' && <>No match yet — keep typing, or paste a user id. </>}
              {users && <>{users.length} people found. </>}
              Used to show only your tasks and to assign the ones you file. Leave empty to see the
              whole database.
            </span>
          </label>
        </section>

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
