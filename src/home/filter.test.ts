import { describe, expect, it } from 'vitest';
import { highlightSegments, matchesQuery, parseQuery } from './filter';

const q = (s: string) => parseQuery(s);

describe('parseQuery', () => {
  it('reads a bare word as free text', () => {
    expect(q('parser').terms).toEqual([{ kind: 'text', value: 'parser' }]);
  });

  it('reads a known key as a field', () => {
    expect(q('provider:github').terms).toEqual([
      { kind: 'field', key: 'provider', values: ['github'], negated: false },
    ]);
  });

  it('splits a comma list into OR values', () => {
    expect(q('provider:github,gitlab').terms).toEqual([
      { kind: 'field', key: 'provider', values: ['github', 'gitlab'], negated: false },
    ]);
  });

  it('accepts - and ! as negation', () => {
    expect(q('-provider:notion').terms[0]).toMatchObject({ negated: true, key: 'provider' });
    expect(q('!provider:notion').terms[0]).toMatchObject({ negated: true, key: 'provider' });
  });

  it('keeps a quoted value in one term', () => {
    expect(q('title:"fix the parser"').terms).toEqual([
      { kind: 'field', key: 'title', values: ['fix the parser'], negated: false },
    ]);
  });

  it('treats an unknown key as free text, sigil included', () => {
    expect(q('foo:bar').terms).toEqual([{ kind: 'text', value: 'foo:bar' }]);
    expect(q('-nope').terms).toEqual([{ kind: 'text', value: '-nope' }]);
  });

  it('ignores a key still being typed', () => {
    expect(q('status:').terms).toEqual([]);
  });

  it('lowercases so matching is case-insensitive', () => {
    expect(q('Provider:GitHub').terms[0]).toMatchObject({ key: 'provider', values: ['github'] });
  });
});

describe('matchesQuery', () => {
  const row = { provider: 'github', priority: 'High', status: 'In progress' };

  it('passes an empty query', () => {
    expect(matchesQuery(q(''), 'anything', row)).toBe(true);
  });

  it('matches a field by substring, case-insensitively', () => {
    expect(matchesQuery(q('priority:hi'), '', row)).toBe(true);
    expect(matchesQuery(q('priority:low'), '', row)).toBe(false);
  });

  it('ANDs distinct keys', () => {
    expect(matchesQuery(q('provider:github priority:high'), '', row)).toBe(true);
    expect(matchesQuery(q('provider:github priority:low'), '', row)).toBe(false);
  });

  it('ORs a repeated key', () => {
    expect(matchesQuery(q('provider:gitlab provider:github'), '', row)).toBe(true);
    expect(matchesQuery(q('provider:gitlab,notion'), '', row)).toBe(false);
  });

  it('excludes on a negated field', () => {
    expect(matchesQuery(q('-provider:github'), '', row)).toBe(false);
    expect(matchesQuery(q('-provider:notion'), '', row)).toBe(true);
  });

  it('searches free text in the haystack', () => {
    expect(matchesQuery(q('parser'), 'TASK-1 fix the parser', row)).toBe(true);
    expect(matchesQuery(q('parser'), 'TASK-1 fix the lexer', row)).toBe(false);
  });

  // The shared-query rule: a field this section does not have must not empty it.
  it('ignores a field the section does not answer', () => {
    expect(matchesQuery(q('owner:alice'), 'anything', row)).toBe(true);
  });

  it('fails a field the section answers with nothing', () => {
    expect(matchesQuery(q('priority:high'), '', { priority: null })).toBe(false);
  });

  it('matches any value of an array field', () => {
    expect(matchesQuery(q('repo:api'), '', { repo: ['web', 'api'] })).toBe(true);
    expect(matchesQuery(q('repo:cli'), '', { repo: ['web', 'api'] })).toBe(false);
  });

  it('matches a boolean field', () => {
    expect(matchesQuery(q('approved:true'), '', { approved: true })).toBe(true);
    expect(matchesQuery(q('approved:false'), '', { approved: true })).toBe(false);
  });
});

describe('highlightSegments', () => {
  const join = (s: string) => highlightSegments(s).map((x) => x.text).join('');

  it('tags a known key and its value', () => {
    expect(highlightSegments('provider:github')).toEqual([
      { text: 'provider:', kind: 'key' },
      { text: 'github', kind: 'value' },
    ]);
  });

  it('keeps the negation sigil with the key', () => {
    expect(highlightSegments('-kind:explorer')[0]).toEqual({ text: '-kind:', kind: 'key' });
  });

  it('leaves an unknown key plain', () => {
    expect(highlightSegments('foo:bar')).toEqual([{ text: 'foo:bar', kind: 'plain' }]);
  });

  it('does not tag mid-word', () => {
    expect(highlightSegments('xtitle:foo')).toEqual([{ text: 'xtitle:foo', kind: 'plain' }]);
  });

  it('keeps a quoted value in one segment', () => {
    expect(highlightSegments('title:"a b"')[1]).toEqual({ text: '"a b"', kind: 'value' });
  });

  // The mirror sits under the real text: any drift misaligns the highlight.
  it('reproduces the input exactly', () => {
    for (const s of ['', 'a', 'provider:github  priority:high', '  -kind:explorer x ', 'title:"a b" z']) {
      expect(join(s)).toBe(s);
    }
  });
});
