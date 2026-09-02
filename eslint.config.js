import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

// Kept to the rules that catch real defects. Formatting is not linted — nothing
// here reformats code, and a style rule that fights an existing file is noise.
export default tseslint.config(
  { ignores: ['dist', 'src-tauri', 'node_modules'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  // Build scripts run under node, not the browser — `console` and friends exist.
  {
    files: ['scripts/**/*.mjs'],
    languageOptions: { globals: { console: 'readonly', process: 'readonly' } },
  },
  {
    files: ['**/*.{ts,tsx}'],
    plugins: { 'react-hooks': reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,

      // Several effects deliberately omit a dependency, each with a comment saying
      // why (identity churn that would rebuild a CodeMirror view). A warning
      // informs; an error would just collect disable directives.
      'react-hooks/exhaustive-deps': 'warn',

      // React Compiler rule. This codebase mirrors props into refs during render
      // (`propsRef.current = props`) in every CodeMirror and xterm host, so the
      // extensions stay stable while handlers still read fresh props — 50+ sites.
      // The rule's remedy is to assign inside an effect, which makes the ref stale
      // for the render that changed the prop. Off, not disabled per-line.
      'react-hooks/refs': 'off',

      // Fires on the standard fetch-in-effect shape used throughout: set loading,
      // await, set data — plus the store-driven effects that seed a modal or move
      // a cursor. The rule's remedy is to not use an effect, which those cases
      // need. Off, not disabled per-line: there were 11, all the same pattern.
      'react-hooks/set-state-in-effect': 'off',

      // Also a React Compiler rule, and it does point at real render-time
      // impurity, so it stays visible.
      'react-hooks/purity': 'warn',

      // tsc already reports unused locals and params; repeating it doubles output.
      '@typescript-eslint/no-unused-vars': 'off',
      // `any` appears where a third-party type is absent or wrong. Everything
      // around it is still checked.
      '@typescript-eslint/no-explicit-any': 'off',
    },
  },
);
