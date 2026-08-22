/** Matching a typed name back to a Notion user id.
 *
 * The picker is a filterable text input rather than a dropdown: a real workspace has
 * hundreds of people, and scrolling a `<select>` to find yourself is worse than
 * typing three letters. That means the typed text has to be resolved back to an id —
 * and a pasted id has to keep working, because a picker that cannot find you must
 * not be a dead end.
 */

import type { NotionUser } from '../ipc/ipc';
export type { NotionUser };

/** What the input offers and echoes: `Name · email`. */
export function userLabel(u: NotionUser): string {
  return u.email ? `${u.name} · ${u.email}` : u.name;
}

/**
 * Notion ids come in two spellings — dashed UUID and bare 32-hex — and both are
 * accepted verbatim so a user can paste one when the list fails them.
 *
 * A PAGE id has the same shape as a user id, so this cannot tell them apart; the
 * caller reports an unmatched id as unverified rather than confirmed.
 */
export function looksLikeNotionId(text: string): boolean {
  const t = text.trim().toLowerCase();
  return /^[0-9a-f]{32}$/.test(t) || /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(t);
}

export type UserMatch =
  /** Nothing typed: no assignee filter, the whole database is shown. */
  | { kind: 'empty' }
  /** Matched a real person in the workspace. */
  | { kind: 'user'; id: string; user: NotionUser }
  /** An id we cannot check — it is not in the list, so it may be a page id. */
  | { kind: 'raw'; id: string }
  | { kind: 'unknown' };

/**
 * Resolve typed text to a user. Tries, in order: the full `Name · email` label, a
 * bare name, an email, an id belonging to someone in the list, then any Notion-
 * shaped id.
 */
export function resolveUser(text: string, users: NotionUser[]): UserMatch {
  const t = text.trim();
  if (!t) return { kind: 'empty' };
  const lower = t.toLowerCase();

  const hit = users.find(
    (u) =>
      userLabel(u).toLowerCase() === lower ||
      u.name.toLowerCase() === lower ||
      u.email?.toLowerCase() === lower ||
      u.id.toLowerCase() === lower ||
      // A dashless paste of a dashed id, and the reverse.
      u.id.replace(/-/g, '').toLowerCase() === lower.replace(/-/g, ''),
  );
  if (hit) return { kind: 'user', id: hit.id, user: hit };

  if (looksLikeNotionId(t)) return { kind: 'raw', id: t };
  return { kind: 'unknown' };
}
