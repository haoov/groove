// Feature dependency direction: a feature imports `shared/` and itself, plus the rows below.
import { dirname, relative, resolve, sep } from 'node:path';

const SRC = resolve(import.meta.dirname, '../src');

const ALLOWED = {
  app: ['*'],
  workspace: ['files', 'git', 'notes', 'editor', 'overview', 'terminal'],
  git: ['editor'],
  settings: ['setup', 'actions'],
  agent: ['setup'],
  overview: ['notes'],
};

const ONLY_FROM = {
  'shared/ipc/generated': 'shared/ipc/ipc.ts',
  '@tauri-apps/api/core': 'shared/ipc/invoke.ts',
};

const STORE_INTERNALS = /^shared\/store\/(slices\/|types$|session$)/;

function srcPath(file) {
  const rel = relative(SRC, file).split(sep).join('/');
  return rel.startsWith('..') ? null : rel;
}

function targetOf(fromFile, source) {
  if (!source.startsWith('.')) return null;
  const abs = resolve(dirname(fromFile), source);
  return srcPath(abs);
}

// A file directly under src/ is an entry point and counts as the composition root.
function featureOf(path) {
  return path.includes('/') ? path.split('/')[0] : 'app';
}

const boundaries = {
  meta: {
    type: 'problem',
    docs: { description: 'enforce the feature dependency direction declared in CLAUDE.md' },
    schema: [],
    messages: {
      feature: "'{{from}}' must not import '{{to}}'. Move the code to shared/ or declare the edge in eslint/boundaries.js.",
      onlyFrom: "'{{source}}' is imported only from {{owner}}.",
      storeInternals: "Import the store barrel 'shared/store', not '{{source}}'.",
    },
  },
  create(context) {
    const file = srcPath(context.filename);
    if (!file) return {};
    const from = featureOf(file);

    function check(node, source) {
      if (typeof source !== 'string') return;

      for (const [needle, owner] of Object.entries(ONLY_FROM)) {
        if (source.includes(needle) && file !== owner) {
          context.report({ node, messageId: 'onlyFrom', data: { source, owner } });
          return;
        }
      }

      const target = targetOf(context.filename, source);
      if (!target) return;
      const to = featureOf(target);

      if (STORE_INTERNALS.test(target.replace(/\.tsx?$/, '')) && !file.startsWith('shared/store/')) {
        context.report({ node, messageId: 'storeInternals', data: { source } });
        return;
      }

      if (to === from || to === 'shared') return;
      const allowed = ALLOWED[from] ?? [];
      if (allowed.includes('*') || allowed.includes(to)) return;
      context.report({ node, messageId: 'feature', data: { from, to } });
    }

    return {
      ImportDeclaration(node) { check(node, node.source.value); },
      ExportAllDeclaration(node) { check(node, node.source?.value); },
      ExportNamedDeclaration(node) { check(node, node.source?.value); },
      ImportExpression(node) {
        if (node.source.type === 'Literal') check(node, node.source.value);
      },
      TSImportType(node) {
        const arg = node.argument;
        if (arg?.type === 'TSLiteralType' && arg.literal?.type === 'Literal') check(node, arg.literal.value);
      },
    };
  },
};

export default { rules: { boundaries } };
