import { describe, expect, it } from 'vitest';
import { StringStream } from '@codemirror/language';
import { gotmplParser } from './gotmpl';

// A stream tokenizer is easy to get subtly wrong: one branch that consumes nothing
// hangs the editor, and a state left set at end-of-line paints the rest of the file
// as a comment. These drive the parser directly, no editor involved.

/** Tokenize one line, carrying state across lines like CodeMirror does. */
function tokenize(lines: string[]) {
  const state = gotmplParser.startState!(2);
  const out: { text: string; tag: string | null }[] = [];
  for (const line of lines) {
    const stream = new StringStream(line, 2, 2);
    let guard = 0;
    while (!stream.eol()) {
      const before = stream.pos;
      const tag = gotmplParser.token(stream, state);
      if (stream.pos === before) throw new Error(`no progress at ${stream.pos} of "${line}"`);
      out.push({ text: line.slice(before, stream.pos), tag });
      if (guard++ > 500) throw new Error('runaway tokenizer');
    }
  }
  return out;
}

const tagOf = (toks: ReturnType<typeof tokenize>, text: string) =>
  toks.find((t) => t.text.trim() === text)?.tag;

describe('gotmpl tokenizer', () => {
  it('always consumes input, on every construct', () => {
    const lines = [
      'name: value', '# comment', '{{ .Values.x }}', '{{- if eq $a "b" -}}',
      '  - item', 'key: "quoted"', 'n: 42', 'b: true', '', '   ', '}}', '{{',
      'weird: @@@ ***', 'a.b/c-d: x', "s: 'single'",
    ];
    expect(() => tokenize(lines)).not.toThrow();
  });

  it('marks a YAML key but not its value', () => {
    const toks = tokenize(['image: nginx']);
    expect(tagOf(toks, 'image')).toBe('property');
    expect(tagOf(toks, 'nginx')).toBeNull();
  });

  it('does not treat a colon inside a value as a key', () => {
    const toks = tokenize(['url: http://example.com']);
    expect(toks.filter((t) => t.tag === 'property').map((t) => t.text.trim())).toEqual(['url']);
  });

  it('marks a list item key', () => {
    expect(tagOf(tokenize(['  - name: web']), '- name')).toBe('property');
  });

  it('brackets the action delimiters and keeps the field path apart', () => {
    const toks = tokenize(['tag: {{ .Values.image.tag }}']);
    expect(toks.some((t) => t.text === '{{' && t.tag === 'bracket')).toBe(true);
    expect(toks.some((t) => t.text === '}}' && t.tag === 'bracket')).toBe(true);
    expect(tagOf(toks, '.Values.image.tag')).toBe('propertyName');
  });

  it('handles the whitespace-trimming delimiters', () => {
    const toks = tokenize(['{{- if .x -}}']);
    expect(toks[0]).toEqual({ text: '{{-', tag: 'bracket' });
    expect(toks[toks.length - 1]).toEqual({ text: '-}}', tag: 'bracket' });
  });

  it('separates keywords, builtins and plain names inside an action', () => {
    const toks = tokenize(['{{ if hasKey .Values "x" }}{{ myVar }}']);
    expect(tagOf(toks, 'if')).toBe('keyword');
    expect(tagOf(toks, 'hasKey')).toBe('builtin');
    expect(tagOf(toks, 'myVar')).toBe('variableName');
  });

  it('marks template variables and pipes', () => {
    const toks = tokenize(['{{ $name | quote }}']);
    expect(tagOf(toks, '$name')).toBe('variableName.special');
    expect(tagOf(toks, '|')).toBe('operator');
    expect(tagOf(toks, 'quote')).toBe('builtin');
  });

  it('keeps a bare scalar whole, so v1 is not read as a number', () => {
    const toks = tokenize(['apiVersion: v1']);
    expect(toks.some((t) => t.text.trim() === 'v1' && t.tag === null)).toBe(true);
    expect(toks.some((t) => t.tag === 'number')).toBe(false);
  });

  it('marks real numbers and atoms', () => {
    expect(tagOf(tokenize(['replicas: 3']), '3')).toBe('number');
    expect(tagOf(tokenize(['enabled: true']), 'true')).toBe('atom');
    expect(tagOf(tokenize(['empty: null']), 'null')).toBe('atom');
  });

  it('marks a YAML comment only at the start of a line', () => {
    expect(tagOf(tokenize(['# a note']), '# a note')).toBe('comment');
    const inline = tokenize(['key: value # trailing']);
    expect(inline.some((t) => t.tag === 'comment')).toBe(false);
  });

  // The dangerous one: a template comment spans lines, so the state must clear at
  // the closing delimiter and not one line later.
  it('carries a multi-line template comment and closes it exactly', () => {
    const toks = tokenize(['{{/*', 'still a comment', '*/}}', 'key: value']);
    expect(toks.filter((t) => t.tag === 'comment').length).toBeGreaterThanOrEqual(3);
    expect(tagOf(toks, 'key')).toBe('property');
  });

  it('closes a single-line template comment', () => {
    const toks = tokenize(['{{/* note */}}', 'key: value']);
    expect(tagOf(toks, 'key')).toBe('property');
  });

  it('leaves an unclosed action open across the line, as Go allows', () => {
    const toks = tokenize(['{{ if .x', '   }}', 'key: value']);
    expect(tagOf(toks, 'if')).toBe('keyword');
    expect(tagOf(toks, 'key')).toBe('property');
  });

  it('reads strings on both sides', () => {
    expect(tagOf(tokenize(['a: "x y"']), '"x y"')).toBe('string');
    expect(tagOf(tokenize(["a: 'x y'"]), "'x y'")).toBe('string');
    expect(tagOf(tokenize(['{{ print "x" }}']), '"x"')).toBe('string');
    expect(tagOf(tokenize(['{{ print `raw` }}']), '`raw`')).toBe('string');
  });

  it('handles an escaped quote without swallowing the line', () => {
    const toks = tokenize(['a: "he said \\"hi\\""', 'key: value']);
    expect(tagOf(toks, 'key')).toBe('property');
  });

  it('tokenizes a realistic chart line', () => {
    const toks = tokenize(['  name: {{ include "chart.fullname" . | trunc 63 | trimSuffix "-" }}']);
    expect(tagOf(toks, 'name')).toBe('property');
    expect(tagOf(toks, 'include')).toBe('builtin');
    expect(tagOf(toks, 'trimSuffix')).toBe('builtin');
    expect(toks.filter((t) => t.tag === 'operator' && t.text === '|')).toHaveLength(2);
  });
});
