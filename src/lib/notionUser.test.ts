import { describe, expect, it } from 'vitest';
import { looksLikeNotionId, resolveUser, userLabel, type NotionUser } from './notionUser';

// Setup writes whatever this resolves into the config as `notion.user_id`, and a
// wrong value there means "no tasks" with no error — Notion happily filters on an id
// that matches nobody. So each branch is pinned.

const arthur: NotionUser = {
  id: '6ef43008-8709-4c8a-9b96-4ca8c92e5f6f',
  name: 'Arthur Le Roux',
  email: 'aleroux@wiremind.io',
};
const prager: NotionUser = {
  id: 'fb092a93-24ea-4550-846f-2d4b5e49a00d',
  name: 'Arthur Prager',
  email: 'aprager@wiremind.io',
};
const noEmail: NotionUser = { id: '11111111-2222-3333-4444-555555555555', name: 'No Email', email: null };
const users = [arthur, prager, noEmail];

describe('userLabel', () => {
  it('shows the email when there is one, to separate two people of the same name', () => {
    expect(userLabel(arthur)).toBe('Arthur Le Roux · aleroux@wiremind.io');
    expect(userLabel(noEmail)).toBe('No Email');
  });
});

describe('resolveUser', () => {
  it('is empty for empty input — no assignee filter at all', () => {
    expect(resolveUser('', users).kind).toBe('empty');
    expect(resolveUser('   ', users).kind).toBe('empty');
  });

  it('matches the offered label', () => {
    const m = resolveUser('Arthur Le Roux · aleroux@wiremind.io', users);
    expect(m).toEqual({ kind: 'user', id: arthur.id, user: arthur });
  });

  it('matches a bare name or an email, case-insensitively', () => {
    expect(resolveUser('arthur le roux', users)).toMatchObject({ id: arthur.id });
    expect(resolveUser('APRAGER@wiremind.io', users)).toMatchObject({ id: prager.id });
  });

  it('keeps two people of the same first name apart', () => {
    expect(resolveUser('Arthur Prager', users)).toMatchObject({ id: prager.id });
    // A prefix alone is ambiguous, so it resolves to nobody rather than guessing.
    expect(resolveUser('Arthur', users).kind).toBe('unknown');
  });

  it('accepts a pasted id belonging to someone in the list, dashed or not', () => {
    expect(resolveUser(arthur.id, users)).toMatchObject({ kind: 'user', id: arthur.id });
    expect(resolveUser(arthur.id.replace(/-/g, ''), users)).toMatchObject({ kind: 'user', id: arthur.id });
  });

  // The reported case: a 32-hex id that is NOT a user (it was a page id). It is
  // accepted, because the picker must not be a dead end — but reported as
  // unverified, so nobody saves it believing it was confirmed.
  it('accepts an unknown Notion-shaped id but marks it unverified', () => {
    const m = resolveUser('227dcbda9c358221ad2201f823195954', users);
    expect(m).toEqual({ kind: 'raw', id: '227dcbda9c358221ad2201f823195954' });
  });

  it('rejects anything that is not a name, an email or an id', () => {
    expect(resolveUser('who?', users).kind).toBe('unknown');
    expect(resolveUser('not-an-id-1234', users).kind).toBe('unknown');
  });
});

describe('looksLikeNotionId', () => {
  it('takes both spellings Notion uses', () => {
    expect(looksLikeNotionId('227dcbda9c358221ad2201f823195954')).toBe(true);
    expect(looksLikeNotionId('6ef43008-8709-4c8a-9b96-4ca8c92e5f6f')).toBe(true);
    expect(looksLikeNotionId('  6EF43008-8709-4C8A-9B96-4CA8C92E5F6F  ')).toBe(true);
  });

  it('rejects near-misses', () => {
    expect(looksLikeNotionId('227dcbda9c358221ad2201f8231959')).toBe(false); // too short
    expect(looksLikeNotionId('zzzdcbda9c358221ad2201f823195954')).toBe(false); // not hex
    expect(looksLikeNotionId('')).toBe(false);
  });
});
