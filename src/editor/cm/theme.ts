import { EditorView } from '@codemirror/view';
import { HighlightStyle } from '@codemirror/language';
import { tags } from '@lezer/highlight';

// ── Catppuccin Frappé syntax highlight (shared by the diff + the editor) ──────
// Hues are referenced through the raw palette tokens (--ctp-*) defined in tokens.css
// so the editor and the rest of the app draw from one source of truth.
export const catppuccinHighlight = HighlightStyle.define([
  { tag: tags.keyword,                   color: 'var(--ctp-mauve)' },
  { tag: [tags.controlKeyword, tags.moduleKeyword], color: 'var(--ctp-mauve)', fontWeight: '600' },
  { tag: tags.string,                    color: 'var(--ctp-green)' },
  { tag: tags.regexp,                    color: 'var(--ctp-green)' },
  { tag: [tags.number, tags.bool, tags.null], color: 'var(--ctp-peach)' },
  { tag: [tags.comment, tags.lineComment, tags.blockComment], color: 'var(--ctp-overlay1)', fontStyle: 'italic' },
  { tag: [tags.function(tags.name), tags.function(tags.variableName)], color: 'var(--ctp-blue)' },
  { tag: tags.definition(tags.function(tags.variableName)), color: 'var(--ctp-blue)' },
  { tag: [tags.typeName, tags.className, tags.namespace, tags.tagName], color: 'var(--ctp-sky)' },
  { tag: tags.definition(tags.typeName),  color: 'var(--ctp-sky)' },
  { tag: tags.operator,                  color: 'var(--ctp-teal)' },
  { tag: tags.propertyName,              color: 'var(--ctp-lavender)' },
  { tag: tags.definition(tags.propertyName), color: 'var(--ctp-lavender)' },
  { tag: [tags.attributeName, tags.attributeValue], color: 'var(--ctp-peach)' },
  { tag: tags.meta,                      color: 'var(--ctp-overlay1)' },
  { tag: tags.link,                      color: 'var(--ctp-blue)', textDecoration: 'underline' },
  { tag: tags.invalid,                   color: 'var(--ctp-red)' },
  { tag: tags.variableName,             color: 'var(--ctp-text)' },
  { tag: tags.punctuation,              color: 'var(--ctp-text)' },
  { tag: tags.paren,                    color: 'var(--ctp-text)' },
  { tag: tags.bracket,                  color: 'var(--ctp-text)' },
  { tag: tags.special(tags.string),     color: 'var(--ctp-teal)' },
]);

// ── Shared CM chrome (both editors) ───────────────────────────────────────────
// Everything that should look identical in the diff and the code editor: base
// background/font, indent markers, selection + search-match highlight, the
// find/replace panel, and the gutter base. Per-editor themes layer cursor/line-metric
// specifics (readonly vs editable) on top.
export const cmChromeTheme = EditorView.theme({
  '&': {
    background: 'transparent',
    fontSize: 'var(--gl-font-size-md)',
    '--indent-marker-bg-color': 'var(--cm-indent-marker)',
    '--indent-marker-active-bg-color': 'var(--cm-indent-marker-active)',
  },
  '&.cm-focused': { outline: 'none' },
  // Lighter base weight (Lilex Light); highlight tags set their own where bolder.
  '.cm-scroller': { fontFamily: 'var(--font-mono)', fontWeight: '300' },
  '.cm-gutters': { background: 'transparent', border: 'none', color: 'var(--gl-text-color-subtle)' },
  '.cm-foldGutter': { display: 'none' },
  '.cm-selectionBackground': { background: 'var(--gl-selection-bg) !important', borderRadius: '3px' },
  '.cm-focused .cm-selectionBackground': { background: 'var(--gl-selection-focus-bg) !important' },
  '.cm-selectionMatch': { background: 'var(--gl-selection-match-bg)', borderRadius: '2px' },
  '.cm-searchMatch': {
    background: 'var(--gl-search-match-bg)',
    outline: '1px solid var(--gl-search-match-border)',
    borderRadius: '2px',
  },
  '.cm-searchMatch.cm-searchMatch-selected': { background: 'var(--gl-search-match-active-bg)' },

  // ── Find / replace panel — compact, Zed-flavoured bar ──────────────────────
  '.cm-panels': {
    background: 'var(--gl-background-color-raised)',
    color: 'var(--gl-text-color-default)',
  },
  '.cm-panels.cm-panels-top': { borderBottom: '1px solid var(--gl-border-color-strong)', boxShadow: 'var(--shadow-float)' },
  '.cm-panels.cm-panels-bottom': { borderTop: '1px solid var(--gl-border-color-strong)', boxShadow: 'var(--shadow-float)' },
  '.cm-search': {
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    rowGap: '8px',
    columnGap: '6px',
    padding: '10px 40px 10px 12px', // room for the absolute close button on the right
    position: 'relative',
    fontFamily: 'var(--font-ui)',
    fontSize: 'var(--gl-font-size-sm)',
  },
  // force a clean two-row break after the search-row controls
  '.cm-search br': { flexBasis: '100%', height: '0', margin: '0', border: 'none' },

  // text fields — match the app's input language
  '.cm-search .cm-textfield': {
    height: '28px',
    boxSizing: 'border-box',
    background: 'var(--gl-background-color-float-inset)',
    border: '1px solid var(--gl-border-color-default)',
    color: 'var(--gl-text-color-default)',
    borderRadius: 'var(--gl-border-radius-medium)',
    padding: '0 10px',
    margin: '0',
    fontFamily: 'var(--font-mono)',
    fontSize: 'var(--gl-font-size-sm)',
    outline: 'none',
    transition: 'border-color 0.15s, box-shadow 0.15s',
  },
  '.cm-search .cm-textfield::placeholder': { color: 'var(--gl-text-color-disabled)' },
  '.cm-search .cm-textfield:focus': {
    borderColor: 'var(--gl-focus-border)',
    boxShadow: 'var(--ring-accent)',
  },
  '.cm-search input[name=search]': { order: '1', minWidth: '220px', flex: '1 1 220px' },
  '.cm-search input[name=replace]': { order: '20', minWidth: '220px', flex: '1 1 220px' },

  // option toggles — render the native checkbox+label pairs as compact glyph pills
  '.cm-search label': {
    order: '2',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    height: '28px',
    minWidth: '30px',
    padding: '0 8px',
    margin: '0',
    fontSize: '0', // hide the native "match case" / "regexp" / "by word" text nodes
    color: 'var(--gl-text-color-subtle)',
    background: 'var(--gl-background-color-strong)',
    border: '1px solid var(--gl-border-color-default)',
    borderRadius: 'var(--gl-border-radius-base)',
    cursor: 'pointer',
    userSelect: 'none',
    fontFamily: 'var(--font-mono)',
    transition: 'background 0.12s, color 0.12s, border-color 0.12s',
  },
  '.cm-search label:hover': { color: 'var(--gl-text-color-default)', borderColor: 'var(--gl-border-color-strong)' },
  '.cm-search label input[type=checkbox]': {
    appearance: 'none', WebkitAppearance: 'none', width: '0', height: '0', margin: '0', position: 'absolute', opacity: '0',
  },
  // glyphs per toggle (via :has on the contained checkbox name)
  '.cm-search label:has(input[name=case])::after':  { content: '"Aa"', fontSize: 'var(--gl-font-size-xs)', fontWeight: '600' },
  '.cm-search label:has(input[name=re])::after':    { content: '".*"', fontSize: 'var(--gl-font-size-sm)', fontWeight: '600' },
  '.cm-search label:has(input[name=word])::after':  { content: '"\\\\b"', fontSize: 'var(--gl-font-size-xs)', fontWeight: '600' },
  // checked = accent fill
  '.cm-search label:has(input:checked)': {
    background: 'var(--gl-color-blue-100)',
    borderColor: 'rgba(140,170,238,0.55)',
    color: 'var(--gl-color-blue-300)',
  },

  // buttons — secondary-button language
  '.cm-search .cm-button': {
    height: '28px',
    boxSizing: 'border-box',
    background: 'var(--gl-background-color-strong)',
    border: '1px solid var(--gl-border-color-default)',
    backgroundImage: 'none',
    color: 'var(--gl-text-color-subtle)',
    borderRadius: 'var(--gl-border-radius-base)',
    padding: '0 12px',
    margin: '0',
    fontFamily: 'var(--font-ui)',
    fontSize: 'var(--gl-font-size-xs)',
    fontWeight: '500',
    cursor: 'pointer',
    transition: 'background 0.12s, color 0.12s, border-color 0.12s',
  },
  '.cm-search .cm-button:hover': { color: 'var(--gl-text-color-default)', borderColor: 'var(--gl-border-color-strong)', background: 'var(--gl-background-color-strong)' },
  '.cm-search .cm-button:active': { background: 'var(--gl-background-color-raised)' },
  // prev / next sit right after the toggles as a tight pair
  '.cm-search button[name=prev]': { order: '5' },
  '.cm-search button[name=next]': { order: '6' },
  '.cm-search button[name=select]': { order: '7' },
  '.cm-search button[name=replace]': { order: '21' },
  '.cm-search button[name=replaceAll]': { order: '22' },

  '.cm-search [name=close]': {
    position: 'absolute',
    top: '10px',
    right: '10px',
    order: '99',
    width: '28px',
    minWidth: '28px',
    padding: '0',
    fontSize: '15px',
    lineHeight: '26px',
    color: 'var(--gl-text-color-subtle)',
  },
  '.cm-search [name=close]:hover': { color: 'var(--gl-text-color-default)' },
});

// ── Editable code-editor specifics (visible cursor; layered over cmChromeTheme) ─
export const editorTheme = EditorView.theme({
  '&': { height: '100%' },
  '.cm-content': { padding: '0', color: 'var(--gl-text-color-default)', caretColor: 'var(--gl-text-color-default)' },
  '.cm-line': { padding: '0px 8px 0 6px', lineHeight: '25px', minHeight: '25px' },
  '.cm-gutterElement': {
    padding: '0',
    lineHeight: '25px',
    fontSize: 'var(--gl-font-size-sm)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  '.cm-activeLine': { background: 'var(--cm-active-line-bg)' },
  '.cm-activeLineGutter': { background: 'var(--cm-active-line-bg)', color: 'var(--gl-text-color-subtle)' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--gl-text-color-default)', borderLeftWidth: '2px' },
});
