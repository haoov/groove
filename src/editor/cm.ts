// CodeMirror 6 wiring for the editor: base extensions, a language picked by
// file extension, and a theme + highlight mapped to the app tokens (Catppuccin).

import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection } from '@codemirror/view';
import type { Extension } from '@codemirror/state';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { syntaxHighlighting, HighlightStyle, indentOnInput, bracketMatching, foldGutter, foldKeymap } from '@codemirror/language';
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
import { tags as t } from '@lezer/highlight';

import { rust } from '@codemirror/lang-rust';
import { javascript } from '@codemirror/lang-javascript';
import { python } from '@codemirror/lang-python';
import { json } from '@codemirror/lang-json';
import { yaml } from '@codemirror/lang-yaml';
import { xml } from '@codemirror/lang-xml';
import { css } from '@codemirror/lang-css';
import { markdown } from '@codemirror/lang-markdown';
import { java } from '@codemirror/lang-java';
import { cpp } from '@codemirror/lang-cpp';
import { sql } from '@codemirror/lang-sql';

/** The language extension for a path, or none for unknown types. */
export function languageFor(path: string): Extension[] {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  switch (ext) {
    case 'rs': return [rust()];
    case 'js': case 'jsx': case 'mjs': case 'cjs': return [javascript()];
    case 'ts': return [javascript({ typescript: true })];
    case 'tsx': return [javascript({ typescript: true, jsx: true })];
    case 'py': return [python()];
    case 'json': return [json()];
    case 'yaml': case 'yml': return [yaml()];
    case 'xml': case 'html': case 'svg': return [xml()];
    case 'css': case 'scss': return [css()];
    case 'md': case 'markdown': return [markdown()];
    case 'java': return [java()];
    case 'c': case 'h': case 'cpp': case 'hpp': case 'cc': return [cpp()];
    case 'sql': return [sql()];
    default: return [];
  }
}

// Colours come from the theme tokens at runtime via CSS custom properties, so the
// editor retints with the app theme without a rebuild.
const v = (name: string) => `var(${name})`;

const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.modifier, t.operatorKeyword], color: v('--gl-color-purple-400') },
  { tag: [t.string, t.special(t.string)], color: v('--gl-color-green-400') },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.definition(t.function(t.variableName))], color: v('--gl-color-blue-400') },
  { tag: [t.number, t.bool, t.atom, t.constant(t.variableName)], color: v('--gl-color-orange-400') },
  { tag: [t.typeName, t.className, t.namespace], color: v('--gl-color-yellow-400') },
  { tag: [t.comment, t.lineComment, t.blockComment], color: v('--gl-text-color-muted'), fontStyle: 'italic' },
  { tag: [t.propertyName, t.attributeName], color: v('--gl-color-teal-400') },
  { tag: [t.tagName], color: v('--gl-color-red-400') },
  { tag: [t.meta, t.macroName], color: v('--gl-color-sapphire-400') },
]);

const theme = EditorView.theme({
  '&': { color: v('--gl-text-color-default'), backgroundColor: 'transparent', height: '100%', fontSize: 'var(--mono-size, 12.5px)' },
  '.cm-scroller': { fontFamily: v('--font-mono'), lineHeight: '1.7', overflow: 'auto' },
  '.cm-content': { caretColor: v('--gl-color-blue-400') },
  '.cm-gutters': { backgroundColor: v('--gl-background-color-overlap'), color: v('--gl-text-color-muted'), border: 'none' },
  '.cm-activeLine': { backgroundColor: v('--cm-active-line-bg') },
  '.cm-activeLineGutter': { backgroundColor: v('--cm-active-line-bg') },
  '.cm-cursor': { borderLeftColor: v('--gl-color-blue-400') },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': { backgroundColor: v('--gl-selection-bg') },
  '.cm-selectionMatch': { backgroundColor: v('--gl-selection-match-bg') },
});

export function baseExtensions(): Extension[] {
  return [
    lineNumbers(),
    highlightActiveLineGutter(),
    foldGutter(),
    history(),
    drawSelection(),
    indentOnInput(),
    bracketMatching(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    syntaxHighlighting(highlight),
    theme,
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, ...foldKeymap, indentWithTab]),
    EditorView.lineWrapping,
  ];
}
