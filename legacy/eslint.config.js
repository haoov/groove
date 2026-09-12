import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';
import groove from './eslint/boundaries.js';

// Defect rules only. Formatting is not linted.
export default tseslint.config(
  { ignores: ['dist', 'src-tauri', 'node_modules'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['scripts/**/*.mjs', 'eslint/**/*.js'],
    languageOptions: { globals: { console: 'readonly', process: 'readonly', URL: 'readonly' } },
  },
  {
    files: ['**/*.{ts,tsx}'],
    plugins: { 'react-hooks': reactHooks, groove },
    rules: {
      ...reactHooks.configs.recommended.rules,

      'groove/boundaries': 'error',

      // Each omitted dependency carries a one-line comment naming what it protects.
      'react-hooks/exhaustive-deps': 'warn',

      // CodeMirror and xterm hosts mirror props into refs during render.
      'react-hooks/refs': 'off',

      // Fetch-in-effect and store-seeded modals set state inside effects.
      'react-hooks/set-state-in-effect': 'off',

      'react-hooks/purity': 'warn',

      // tsc already reports unused locals and parameters.
      '@typescript-eslint/no-unused-vars': 'off',
      // `any` marks an absent or wrong third-party type.
      '@typescript-eslint/no-explicit-any': 'off',
    },
  },
);
