import { LanguageSupport, StreamLanguage, type StreamParser } from '@codemirror/language';

/**
 * Go templates — Helm charts, `.tpl`, `.gotmpl`.
 *
 * A `StreamLanguage` rather than a Lezer grammar: the content is two languages at
 * once (YAML with `{{ }}` cut into it, often mid-structure), which a real parser
 * cannot represent — `{{- if }}` can open a block that closes three keys later, so
 * the YAML never parses on its own. A tokenizer does not care, and highlighting is
 * what is actually wanted here.
 *
 * Outside an action it does YAML-lite (keys, strings, comments) so a chart template
 * still reads as YAML. Inside one it highlights the template language properly.
 * No dependency and no language server; hover and completion would need an LSP,
 * which is a separate piece of work.
 */

/** Control flow and the built-in actions. */
const KEYWORDS = new Set([
  'if', 'else', 'end', 'range', 'with', 'define', 'template', 'block',
  'break', 'continue', 'return', 'nil', 'and', 'or', 'not',
  'eq', 'ne', 'lt', 'le', 'gt', 'ge',
]);

/** The functions actually typed in these repos: Go's builtins plus the sprig and
 *  Helm helpers that show up in every chart. */
const BUILTINS = new Set([
  'len', 'index', 'slice', 'print', 'printf', 'println', 'call', 'html', 'js', 'urlquery',
  'include', 'required', 'tpl', 'lookup', 'fail',
  'toYaml', 'toJson', 'fromYaml', 'fromJson', 'indent', 'nindent', 'quote', 'squote',
  'default', 'empty', 'coalesce', 'ternary', 'trunc', 'trim', 'trimSuffix', 'trimPrefix',
  'lower', 'upper', 'title', 'replace', 'splitList', 'join', 'contains', 'hasPrefix', 'hasSuffix',
  'dict', 'list', 'get', 'set', 'unset', 'hasKey', 'keys', 'values', 'merge', 'deepCopy',
  'b64enc', 'b64dec', 'sha256sum', 'randAlphaNum', 'uuidv4',
  'semverCompare', 'regexMatch', 'now', 'date', 'kindIs', 'typeOf',
]);

interface State {
  /** Between `{{` and `}}`. */
  inAction: boolean;
  /** Inside a `{{/* … *\/}}` comment, which may span lines. */
  inComment: boolean;
}

/** The raw stream parser. Exported so it can be driven directly in a test with a
 *  `StringStream`; the app uses `gotmpl()` below. */
export const gotmplParser: StreamParser<State> = {
  name: 'gotmpl',
  startState: () => ({ inAction: false, inComment: false }),

  token(stream, state) {
    // ── Template comments, possibly multi-line ────────────────────────────────
    if (state.inComment) {
      if (stream.match(/^[\s\S]*?\*\/\s*-?\}\}/)) {
        state.inComment = false;
        state.inAction = false;
      } else {
        stream.skipToEnd();
      }
      return 'comment';
    }

    // ── Outside an action: YAML-lite ──────────────────────────────────────────
    if (!state.inAction) {
      if (stream.match(/^\{\{-?\s*\/\*/)) {
        state.inComment = true;
        return 'comment';
      }
      if (stream.match(/^\{\{-?/)) {
        state.inAction = true;
        return 'bracket';
      }
      if (stream.sol() && stream.match(/^\s*#.*/)) return 'comment';
      // A YAML key: `name:` / `- name:` — only before the colon.
      if (stream.match(/^\s*-?\s*[A-Za-z_][\w.\-/]*(?=\s*:(\s|$))/)) return 'property';
      if (stream.match(/^"(?:[^"\\]|\\.)*"/) || stream.match(/^'(?:[^'\\]|\\.)*'/)) return 'string';
      if (stream.match(/^-?\d+(\.\d+)?\b/)) return 'number';
      if (stream.match(/^(true|false|null|~)\b/)) return 'atom';
      // A bare scalar (`v1`, `my-app`) is consumed whole: matching the number
      // first highlighted the `1` inside `v1`.
      if (stream.match(/^[A-Za-z_][\w.\-/]*/)) return null;
      // Skip a run of characters that cannot begin any of the above, so ordinary
      // prose does not cost one token per character.
      if (stream.match(/^[^{#"'\w\-\d]+/)) return null;
      stream.next();
      return null;
    }

    // ── Inside an action ──────────────────────────────────────────────────────
    if (stream.match(/^-?\}\}/)) {
      state.inAction = false;
      return 'bracket';
    }
    if (stream.eatSpace()) return null;
    if (stream.match(/^"(?:[^"\\]|\\.)*"/) || stream.match(/^`[^`]*`/)) return 'string';
    // `$`, `$name` — template variables.
    if (stream.match(/^\$[\w]*/)) return 'variableName.special';
    // `.`, `.Values.image.tag` — the data pipeline's field paths.
    if (stream.match(/^\.[\w.]*/)) return 'propertyName';
    if (stream.match(/^-?\d+(\.\d+)?\b/)) return 'number';
    if (stream.match(/^\|/)) return 'operator';
    if (stream.match(/^:?=/)) return 'operator';
    if (stream.match(/^[(),]/)) return 'punctuation';

    const word = stream.match(/^[A-Za-z_]\w*/) as RegExpMatchArray | null;
    if (word) {
      if (KEYWORDS.has(word[0])) return 'keyword';
      if (BUILTINS.has(word[0])) return 'builtin';
      return 'variableName';
    }

    stream.next();
    return null;
  },

  languageData: {
    commentTokens: { block: { open: '{{/*', close: '*/}}' } },
  },
};

export const gotmplLanguage = StreamLanguage.define(gotmplParser);

export function gotmpl(): LanguageSupport {
  return new LanguageSupport(gotmplLanguage);
}
