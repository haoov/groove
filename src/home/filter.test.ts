import { describe, expect, it } from 'vitest';
import {
  applySuggestion, appliesTo, highlightSegments, matchesQuery, parseQuery, queryKeys, suggest,
} from './filter';

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

  // A field the section cannot answer excludes every row, so the tab reads as
  // empty and Home routes the query to the tab that owns the field.
  it('excludes every row for a field the section does not answer', () => {
    expect(matchesQuery(q('owner:alice'), 'anything', row)).toBe(false);
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

describe('suggest', () => {
  const values = { provider: ['github', 'gitlab', 'notion'], repo: ['api', 'web app'] };
  const at = (s: string, values2 = values) => suggest(s, s.length, values2);

  it('offers every key on an empty query', () => {
    const r = at('');
    expect(r.kind).toBe('key');
    expect(r.items.map((i) => i.label)).toContain('provider:');
  });

  it('narrows keys by prefix', () => {
    expect(at('pri').items.map((i) => i.label)).toEqual(['priority:']);
  });

  it('offers a key its values once the colon is typed', () => {
    const r = at('provider:');
    expect(r.kind).toBe('value');
    expect(r.items.map((i) => i.label)).toEqual(['github', 'gitlab', 'notion']);
  });

  it('narrows values by substring', () => {
    expect(at('provider:git').items.map((i) => i.label)).toEqual(['github', 'gitlab']);
  });

  it('keeps the negation sigil in the completion', () => {
    expect(at('-provider:git').items[0].insert).toBe('-provider:github');
  });

  it('quotes a value that has spaces', () => {
    expect(at('repo:web').items[0].insert).toBe('repo:"web app"');
  });

  it('completes the token at the caret, not the whole query', () => {
    const s = 'provider:github pri';
    const r = suggest(s, s.length, values);
    expect(r.start).toBe(16);
    expect(r.items.map((i) => i.label)).toEqual(['priority:']);
  });

  it('offers nothing for a key with no loaded values', () => {
    expect(at('status:').items).toEqual([]);
  });
});

describe('applySuggestion', () => {
  it('splices the completion in and reports the caret', () => {
    expect(applySuggestion('provider:github pri', 16, 19, 'priority:'))
      .toEqual({ text: 'provider:github priority:', caret: 25 });
  });

  it('replaces mid-query without touching the rest', () => {
    expect(applySuggestion('a bb c', 2, 4, 'XYZ')).toEqual({ text: 'a XYZ c', caret: 5 });
  });
});

describe('appliesTo', () => {
  const FIELDS = ['id', 'title', 'status', 'priority', 'provider'];

  it('applies when the query names no field', () => {
    expect(appliesTo(parseQuery('some text'), FIELDS)).toBe(true);
  });

  it('applies when every named field is answerable', () => {
    expect(appliesTo(parseQuery('priority:high provider:notion'), FIELDS)).toBe(true);
  });

  it('does not apply when any named field is missing', () => {
    expect(appliesTo(parseQuery('priority:high owner:alice'), FIELDS)).toBe(false);
  });

  it('ignores an unknown key, which parses as free text', () => {
    expect(appliesTo(parseQuery('foo:bar'), FIELDS)).toBe(true);
  });
});

describe('queryKeys', () => {
  it('lists each named field once', () => {
    expect(queryKeys(parseQuery('provider:a provider:b title:x free'))).toEqual(['provider', 'title']);
  });
});

// `provider` is where the TASK came from; `forge` is where the CODE is hosted. An
// MR has no provider. Both can read "github", so one key for the two would answer
// the wrong question — these lock the split.
describe('provider and forge are separate axes', () => {
  const REVIEW_FIELDS = ['id', 'mr', 'title', 'forge', 'repo', 'owner'];
  const TASK_FIELDS = ['id', 'title', 'status', 'priority', 'provider'];

  it('does not let a provider query claim the reviews tab', () => {
    expect(appliesTo(parseQuery('provider:notion'), REVIEW_FIELDS)).toBe(false);
  });

  it('does not let a forge query claim the tasks tab', () => {
    expect(appliesTo(parseQuery('forge:gitlab'), TASK_FIELDS)).toBe(false);
  });

  it('keeps github distinct between the two keys', () => {
    const mr = { forge: 'github', title: 'fix' };
    const task = { provider: 'github', title: 'fix' };
    expect(matchesQuery(parseQuery('forge:github'), '', mr)).toBe(true);
    expect(matchesQuery(parseQuery('provider:github'), '', mr)).toBe(false);
    expect(matchesQuery(parseQuery('provider:github'), '', task)).toBe(true);
    expect(matchesQuery(parseQuery('forge:github'), '', task)).toBe(false);
  });

  it('offers forge as its own field', () => {
    expect(suggest('for', 3, {}).items.map((i) => i.label)).toEqual(['forge:']);
  });
});
