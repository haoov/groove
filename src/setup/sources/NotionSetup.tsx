import { useEffect, useState } from 'react';
import { Check, Loader2, Minus } from 'lucide-react';
import { invoke } from '../../shared/ipc/invoke';
import { DetectedPanel } from './DetectedPanel';
import { looksLikeNotionId } from '../../shared/lib/notionUser';
import type { DetectedSchema, NotionUser } from '../../shared/ipc/ipc';
import type { SetupFormProps, SettingsRowProps } from './index';

/** The values only the user can supply: a token, which database, who they are,
 *  and optionally a template page. Everything else is detected on save. */
export function NotionSetupForm({ onChange }: SetupFormProps) {
  const [token, setToken] = useState('');
  const [databaseId, setDatabaseId] = useState('');
  const [who, setWho] = useState('');
  const [found, setFound] = useState<NotionUser | null>(null);
  const [template, setTemplate] = useState('');
  const [detected, setDetected] = useState<DetectedSchema | null>(null);
  const [busy, setBusy] = useState<'users' | 'detect' | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Empty → no assignee filter; a found user or a pasted Notion id → that id.
  const whoMatch = !who.trim()
    ? { kind: 'empty' as const }
    : found
      ? { kind: 'user' as const, id: found.id, user: found }
      : looksLikeNotionId(who)
        ? { kind: 'raw' as const, id: who.trim() }
        : { kind: 'unknown' as const };

  // The parent only decides "can this save"; the payload carries the rest.
  const filled = !!token.trim() && !!databaseId.trim();
  useEffect(() => {
    onChange(filled ? {
      token: token.trim(),
      database_id: databaseId.trim(),
      user_id: whoMatch.kind === 'user' || whoMatch.kind === 'raw' ? whoMatch.id : '',
      template_page_id: template.trim() || null,
    } : null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, databaseId, who, found, template]);

  // The token is checked by using it: looking yourself up by email is both the
  // check and the way to get your user id without hunting a UUID in the API.
  const lookupUser = async () => {
    if (!token.trim() || !who.trim() || looksLikeNotionId(who)) return;
    setBusy('users');
    setError(null);
    try {
      setFound(await invoke<NotionUser>('find_notion_user', { token: token.trim(), email: who.trim() }));
    } catch (e) {
      setFound(null);
      // Could be a bad token OR a missing capability OR an unknown email — the
      // backend message says which.
      setError(String(e));
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
      setDetected(await invoke<DetectedSchema>('detect_notion_database', {
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

  return (
    <>
      <label className="firstrun-field">
        <span className="firstrun-label">Integration token</span>
        <input
          className="firstrun-input"
          type="password"
          autoFocus
          placeholder="ntn_…"
          value={token}
          onChange={(e) => setToken(e.target.value)}
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
        <DetectedPanel
          detected={detected}
          note="Detected from the property types and Notion's own status groups."
        />
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
            placeholder={token.trim() ? 'Your Notion email, or paste a user id' : 'Enter the token first'}
            value={who}
            onChange={(e) => { setWho(e.target.value); setFound(null); }}
            onKeyDown={(e) => { if (e.key === 'Enter') lookupUser(); }}
          />
          <button
            className="btn-secondary"
            onClick={lookupUser}
            disabled={!token.trim() || !who.trim() || looksLikeNotionId(who) || busy !== null}
          >
            {busy === 'users' ? <Loader2 size={11} className="spin" /> : null}
            find me
          </button>
        </div>
        <span className="firstrun-hint">
          {whoMatch.kind === 'user' && (
            <>Matched <strong>{whoMatch.user.name}</strong> · <code>{whoMatch.id}</code>. </>
          )}
          {whoMatch.kind === 'raw' && (
            <>Using <code>{whoMatch.id}</code> as-is — it cannot be checked, so make sure it is
              a USER id and not a page id. </>
          )}
          {whoMatch.kind === 'unknown' && <>Type your email and press find me, or paste a user id. </>}
          Used to show only your tasks and to assign the ones you file. Leave empty to see the
          whole database.
        </span>
      </label>

      {error && <div className="firstrun-warn"><span>{error}</span></div>}
    </>
  );
}

/** Settings: connected state + disconnect. Adding Notion after first run means
 *  the token flow, which stays in the config file for now. */
export function NotionSettingsRow({ config, busy, setSource }: SettingsRowProps) {
  const on = !!config?.notion;
  return (
    <li className={on ? 'ok' : 'optional'}>
      {on ? <Check size={12} strokeWidth={2.5} /> : <Minus size={12} strokeWidth={2} />}
      <code>notion</code>
      <span className="firstrun-tool-purpose">
        {config?.notion
          ? `database ${config.notion.database_id.slice(-8)}`
          : 'not set up — add it in the config file'}
      </span>
      {on && (
        <button className="firstrun-signin" disabled={busy} onClick={() => setSource(false, null)}>
          {busy ? <Loader2 size={11} className="spin" /> : 'disconnect'}
        </button>
      )}
    </li>
  );
}
