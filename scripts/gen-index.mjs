// Rebuild the barrel for the ts-rs generated types. Run by `pnpm gen:types`.
import { readdirSync, writeFileSync } from 'node:fs';

const dir = 'src/shared/ipc/generated';
const types = readdirSync(dir)
  .filter((f) => f.endsWith('.ts') && f !== 'index.ts')
  .map((f) => f.replace(/\.ts$/, ''))
  .sort();

const body =
  '// Generated barrel for the ts-rs types — do not edit. Run `pnpm gen:types`.\n' +
  types.map((t) => `export type { ${t} } from './${t}';`).join('\n') +
  '\n';

writeFileSync(`${dir}/index.ts`, body);
console.log(`barrel: ${types.length} types`);
