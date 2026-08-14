export function fileIconColor(name: string): string {
  const ext = (name.split('/').pop() ?? name).split('.').pop()?.toLowerCase() ?? '';
  const colors: Record<string, string> = {
    ts: '#3178c6',   tsx: '#61dafb',
    js: '#f7df1e',   jsx: '#61dafb',
    rs: '#ce422b',
    py: '#3572a5',
    go: '#00add8',
    html: '#e34c26', htm: '#e34c26',
    css: '#264de4',  scss: '#c6538c',  sass: '#c6538c',
    md: '#8b949e',   mdx: '#8b949e',
    json: '#cbcb41',
    rb: '#cc342d',
    java: '#b07219',
    kt: '#7f52ff',
    swift: '#f05138',
    sql: '#e38c00',
  };
  return colors[ext] ?? '#6e7681';
}
