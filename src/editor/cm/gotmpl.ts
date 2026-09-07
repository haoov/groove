import { LanguageSupport, StreamLanguage, type StreamParser } from '@codemirror/language';

/**
 * Go template highlighting for Helm charts, `.tpl` and `.gotmpl`: a `StreamLanguage`
 * that tokenizes YAML-lite outside `{{ }}` and the template language inside.
 */

/** Control flow and the built-in actions. */
const KEYWORDS = new Set([
  'if', 'else', 'end', 'range', 'with', 'define', 'template', 'block',
  'break', 'continue', 'return', 'nil', 'and', 'or', 'not',
  'eq', 'ne', 'lt', 'le', 'gt', 'ge',
]);

/** Go builtins plus the sprig and Helm helpers. */
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

/** The raw stream parser; tests drive it directly with a `StringStream`. */
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
      // A bare scalar (`v1`, `my-app`), consumed whole.
      if (stream.match(/^[A-Za-z_][\w.\-/]*/)) return null;
      // A run of characters that cannot begin a token.
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
