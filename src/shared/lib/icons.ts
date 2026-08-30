/** File-icon tint, by extension. Palette tokens, so icons re-colour with the theme. */
const COLORS: Record<string, string> = {
  'chart.yaml': 'lavender', 'values.yaml': 'lavender', tpl: 'lavender',

  ts: 'blue', mts: 'blue', cts: 'blue',
  tsx: 'sky', jsx: 'sky',
  js: 'yellow', mjs: 'yellow', cjs: 'yellow', json: 'yellow', jsonc: 'yellow',
  rs: 'peach',
  py: 'teal', pyi: 'teal',
  go: 'sapphire',
  rb: 'red',
  java: 'maroon', kt: 'mauve', kts: 'mauve',
  swift: 'flamingo',
  php: 'lavender',
  vue: 'green', svelte: 'maroon',
  dart: 'sapphire',
  lua: 'blue',
  c: 'sapphire', h: 'sapphire', cpp: 'blue', cc: 'blue', hpp: 'blue',
  html: 'peach', htm: 'peach',
  css: 'blue', scss: 'pink', sass: 'pink',
  md: 'subtext0', mdx: 'subtext0',
  yaml: 'mauve', yml: 'mauve', toml: 'mauve',
  env: 'yellow',
  sh: 'green', bash: 'green', zsh: 'green',
  graphql: 'pink', gql: 'pink',
  tf: 'mauve', tfvars: 'mauve',
  sql: 'peach',
  dockerfile: 'sapphire',
  gitignore: 'peach', gitattributes: 'peach', gitmodules: 'peach',
};

/** `.env` → `env`, `Dockerfile` → `dockerfile`, `main.rs` → `rs`. */
export function iconKey(name: string): string {
  const base = (name.split('/').pop() ?? name).toLowerCase();
  return base.startsWith('.') ? base.slice(1) : (base.split('.').pop() ?? '');
}

export function fileIconColor(name: string): string {
  const base = (name.split('/').pop() ?? name).toLowerCase();
  return `var(--ctp-${COLORS[base] ?? COLORS[iconKey(name)] ?? 'overlay2'})`;
}
