const LANG_MAP: Record<string, string> = {
  ts: 'typescript', tsx: 'typescript', js: 'javascript', jsx: 'javascript',
  rs: 'rust', go: 'go', py: 'python', rb: 'ruby', java: 'java',
  c: 'c', cpp: 'cpp', cs: 'csharp', kt: 'kotlin', swift: 'swift',
  json: 'json', yaml: 'yaml', yml: 'yaml', toml: 'toml',
  md: 'markdown', html: 'html', css: 'css', scss: 'scss',
  sh: 'shell', bash: 'shell', sql: 'sql', dockerfile: 'dockerfile',
  tpl: 'gotmpl', gotmpl: 'gotmpl', tmpl: 'gotmpl',
};

/** A Helm chart's `templates/*.yaml` is Go template, not YAML: `{{- if }}` opens
 *  blocks the YAML parser cannot close, so YAML highlighting mangles them. */
function isHelmTemplate(path: string, ext: string): boolean {
  return (ext === 'yaml' || ext === 'yml') && /(^|\/)templates\//.test(path);
}

export function guessLang(path: string): string {
  const ext = (path.split('.').pop() ?? '').toLowerCase();
  if (isHelmTemplate(path, ext)) return 'gotmpl';
  return LANG_MAP[ext] ?? 'plaintext';
}
