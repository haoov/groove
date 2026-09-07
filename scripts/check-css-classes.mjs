#!/usr/bin/env node
// Fails when a class selector in src/**/*.css is referenced by no .ts/.tsx file.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const ROOT = new URL('../src', import.meta.url).pathname;

// Class prefixes built at runtime from data, and classes owned by third-party DOM.
const DYNAMIC_PREFIXES = [
  'status-', 'mr-state-', 'st-', 'forge-ci-', 'resize-grip-', 'notif--', 'type-',
  'lvl-', 'flt-', 'cm-', 'xterm', 'ͼ',
];
const DYNAMIC_EXACT = new Set(['p0', 'p1']);

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      if (name !== 'generated') walk(p, out);
    } else {
      out.push(p);
    }
  }
  return out;
}

const files = walk(ROOT);
const cssFiles = files.filter((f) => f.endsWith('.css'));
const codeFiles = files.filter((f) => /\.tsx?$/.test(f) && !/\.test\.tsx?$/.test(f));

const code = codeFiles.map((f) => readFileSync(f, 'utf8')).join('\n');
const codeTokens = new Set(code.match(/[A-Za-z0-9_-]+/g) ?? []);

const defined = new Map();
for (const f of cssFiles) {
  const css = readFileSync(f, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
  let m;
  const re = /\.(-?[_a-zA-Z][\w-]*)/g;
  while ((m = re.exec(css))) {
    const cls = m[1];
    if (!defined.has(cls)) defined.set(cls, relative(ROOT, f));
  }
}

const isDynamic = (cls) =>
  DYNAMIC_EXACT.has(cls) || DYNAMIC_PREFIXES.some((p) => cls.startsWith(p));

const unused = [...defined]
  .filter(([cls]) => !codeTokens.has(cls) && !isDynamic(cls))
  .sort(([a], [b]) => a.localeCompare(b));

if (unused.length) {
  console.error(`${unused.length} CSS class(es) referenced by no .ts/.tsx file:`);
  for (const [cls, file] of unused) console.error(`  .${cls}  (${file})`);
  process.exit(1);
}
console.log(`css classes: ${defined.size} defined, all referenced`);
