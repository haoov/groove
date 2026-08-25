// Home's filter grammar: `field:value` tokens plus bare words, in the shape
// GitHub and GitLab use. One parser, three field sets — each section declares
// the fields it can answer and the free text it searches.
//
//   provider:github priority:high        both must hold
//   provider:github,gitlab               same key twice reads as OR
//   -provider:notion                     `-` or `!` negates
//   title:"fix the parser"               quotes carry spaces
//   parser                               a bare word searches the whole row
//
// The query is shared across the tabs, so a section is asked about fields it
// does not have. Two rules keep that harmless: a key another section owns is
// IGNORED here (it must not empty the list), while a key no section owns is a
// typo and falls back to free text.

export type Term =
  | { kind: 'text'; value: string }
  | { kind: 'field'; key: string; values: string[]; negated: boolean };

export interface Query {
  terms: Term[];
}

/** Every field any section answers. A key outside this set is treated as text. */
export const KNOWN_KEYS = new Set([
  'id', 'title', 'provider', 'forge', 'kind', 'status', 'priority',
  'repo', 'branch', 'owner', 'author', 'approved', 'draft', 'mr',
]);

/**
 * What each field means, shown beside the key in the suggestion list.
 *
 * `provider` and `forge` are different axes and must never be merged: a provider
 * is where the TASK came from, a forge is where the CODE is hosted. An MR has no
 * provider at all. Both can read "github", which is exactly why one key for the
 * two would answer the wrong question.
 */
export const KEY_HELP: Record<string, string> = {
  id: 'task id or MR number',
  title: 'name of the task or MR',
  provider: 'where the task came from — notion · github',
  forge: 'where the code is hosted — github · gitlab',
  kind: 'task · explorer · review',
  status: 'workflow status',
  priority: 'priority as the source names it',
  repo: 'repository',
  branch: 'branch name',
  owner: 'who opened it',
  author: 'who opened it',
  approved: 'true · false',
  draft: 'true · false',
  mr: 'MR number',
};

export interface Suggestion {
  kind: 'key' | 'value';
  /** Shown in the list. */
  label: string;
  /** Replaces the token being typed. */
  insert: string;
  /** The field this line belongs to — the list reads it for the icon. */
  field: string;
  hint?: string;
}

export interface SuggestResult {
  items: Suggestion[];
  /** Range of the token the completion replaces. */
  start: number;
  end: number;
  kind: 'key' | 'value';
}

// Values are unbounded (every repo, every author), so cap them and let the list
// scroll. Keys are a fixed set of thirteen — truncating those would hide fields
// nobody can then discover.
const MAX_VALUES = 10;

/** The whitespace-bounded token around the caret. */
export function tokenRange(input: string, caret: number): [number, number] {
  let start = caret;
  while (start > 0 && !/\s/.test(input[start - 1])) start--;
  let end = caret;
  while (end < input.length && !/\s/.test(input[end])) end++;
  return [start, end];
}

const quoted = (v: string) => (/\s/.test(v) ? `"${v}"` : v);

/**
 * What to offer for the token under the caret: values once a known `key:` is
 * typed, otherwise the field names themselves. `values` supplies what the loaded
 * rows actually contain, so the list only ever offers something that matches.
 */
export function suggest(input: string, caret: number, values: Record<string, string[]>): SuggestResult {
  const [start, end] = tokenRange(input, caret);
  const token = input.slice(start, end);
  const sigil = /^[-!]/.test(token) ? token[0] : '';
  const bare = sigil ? token.slice(1) : token;
  const colon = bare.indexOf(':');

  if (colon >= 0) {
    const key = bare.slice(0, colon).toLowerCase();
    if (KNOWN_KEYS.has(key)) {
      const partial = bare.slice(colon + 1).replace(/"/g, '').toLowerCase();
      const items = (values[key] ?? [])
        .filter((v) => v.toLowerCase().includes(partial))
        .slice(0, MAX_VALUES)
        .map((v): Suggestion => ({ kind: 'value', label: v, insert: `${sigil}${key}:${quoted(v)}`, field: key }));
      return { items, start, end, kind: 'value' };
    }
  }

  const partial = bare.toLowerCase();
  const items = [...KNOWN_KEYS]
    .filter((k) => k.startsWith(partial))
    .sort()
    .map((k): Suggestion => ({ kind: 'key', label: `${k}:`, insert: `${sigil}${k}:`, field: k, hint: KEY_HELP[k] }));
  return { items, start, end, kind: 'key' };
}

/** Splice a completion in, and say where the caret lands. */
export function applySuggestion(
  input: string, start: number, end: number, insert: string,
): { text: string; caret: number } {
  return { text: input.slice(0, start) + insert + input.slice(end), caret: start + insert.length };
}

/** A run of the raw query, tagged so the input's mirror layer can colour it. */
export interface Segment {
  text: string;
  kind: 'plain' | 'key' | 'value';
}

const TOKEN_RE = /([-!]?)([A-Za-z_]+):("[^"]*"|\S*)/g;

/**
 * Split the raw query into display runs: a recognised `key:` and its value get
 * their own segments, everything else stays plain. Concatenating the segments
 * reproduces the input exactly — the mirror must line up with the real text.
 */
export function highlightSegments(input: string): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  for (const m of input.matchAll(TOKEN_RE)) {
    const start = m.index;
    // Only at a token boundary: "xtitle:foo" is not a field.
    if (start > 0 && !/\s/.test(input[start - 1])) continue;
    if (!KNOWN_KEYS.has(m[2].toLowerCase())) continue;
    if (start > last) out.push({ text: input.slice(last, start), kind: 'plain' });
    out.push({ text: `${m[1]}${m[2]}:`, kind: 'key' });
    if (m[3]) out.push({ text: m[3], kind: 'value' });
    last = start + m[0].length;
  }
  if (last < input.length) out.push({ text: input.slice(last), kind: 'plain' });
  return out;
}

/**
 * How a tab reports back to Home: how many rows matched, whether the query even
 * applies here, and which filter the answer is for (Home routes only once every
 * tab has answered for the current query).
 */
export type CountReport = (n: number, applicable: boolean, forFilter: string) => void;

/** What a section can answer: field name → the value(s) of the row. */
export type Fields = Record<string, string | number | boolean | null | undefined | string[]>;

const KEY_RE = /^([a-z_]+):(.*)$/i;

/** Split on whitespace, but keep a double-quoted run together. */
function tokenize(input: string): string[] {
  const out: string[] = [];
  let buf = '';
  let quoted = false;
  for (const ch of input) {
    if (ch === '"') { quoted = !quoted; continue; }
    if (!quoted && /\s/.test(ch)) {
      if (buf) { out.push(buf); buf = ''; }
      continue;
    }
    buf += ch;
  }
  if (buf) out.push(buf);
  return out;
}

export function parseQuery(input: string): Query {
  const terms: Term[] = [];
  for (const raw of tokenize(input)) {
    let token = raw;
    let negated = false;
    if (token.startsWith('-') || token.startsWith('!')) {
      negated = true;
      token = token.slice(1);
    }
    const m = KEY_RE.exec(token);
    // A typo'd key stays free text — and keeps the sigil it came with, since a
    // bare `-foo` is a search for "-foo", not a negation of nothing.
    if (!m || !KNOWN_KEYS.has(m[1].toLowerCase())) {
      terms.push({ kind: 'text', value: raw.toLowerCase() });
      continue;
    }
    const values = m[2].split(',').map((v) => v.trim().toLowerCase()).filter(Boolean);
    // `status:` with nothing after it is still being typed; ignore it.
    if (values.length === 0) continue;
    terms.push({ kind: 'field', key: m[1].toLowerCase(), values, negated });
  }
  return { terms };
}

/** True when the query has nothing to apply. */
export const isEmptyQuery = (q: Query) => q.terms.length === 0;

/** The distinct fields the query constrains. */
export const queryKeys = (q: Query): string[] =>
  [...new Set(q.terms.flatMap((t) => (t.kind === 'field' ? [t.key] : [])))];

/**
 * True when a section can answer every field the query names. A section that
 * cannot is not merely empty — it is the wrong tab for this query, which is what
 * lets Home tell "nothing matched here" apart from "ask another tab".
 */
export const appliesTo = (q: Query, fields: readonly string[]): boolean =>
  queryKeys(q).every((k) => fields.includes(k));

function haystack(v: Fields[string]): string[] {
  if (v === null || v === undefined) return [];
  if (Array.isArray(v)) return v.map((s) => String(s).toLowerCase());
  return [String(v).toLowerCase()];
}

/** Substring, case-insensitive — `prio:hi` should find "High". */
const hits = (values: string[], against: string[]) =>
  values.some((v) => against.some((a) => a.includes(v)));

/**
 * `text` is the row's free-text haystack (id, title, whatever the section shows).
 * `fields` answers the typed keys. Repeated keys OR together; distinct keys AND.
 */
export function matchesQuery(q: Query, text: string, fields: Fields): boolean {
  const hay = text.toLowerCase();
  // Group positives by key so `provider:a provider:b` reads as OR.
  const positives = new Map<string, string[]>();
  for (const t of q.terms) {
    if (t.kind === 'text') {
      if (!hay.includes(t.value)) return false;
      continue;
    }
    // A field this section cannot answer excludes every row: the tab is not
    // applicable, and Home routes the query to the tab that owns the field.
    if (!(t.key in fields)) return false;
    if (t.negated) {
      if (hits(t.values, haystack(fields[t.key]))) return false;
      continue;
    }
    const prev = positives.get(t.key);
    if (prev) prev.push(...t.values); else positives.set(t.key, [...t.values]);
  }
  for (const [key, values] of positives) {
    if (!hits(values, haystack(fields[key]))) return false;
  }
  return true;
}
