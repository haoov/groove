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
  'id', 'title', 'provider', 'kind', 'status', 'priority',
  'repo', 'branch', 'owner', 'author', 'approved', 'draft', 'mr',
]);

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
    if (!(t.key in fields)) continue; // another section's field — not our business
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
