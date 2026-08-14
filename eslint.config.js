import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

// Kept to the rules that catch real defects. Formatting is not linted — nothing
// here reformats code, and a style rule that fights an existing file is noise.
export default tseslint.config(
  { ignores: ['dist', 'src-tauri', 'node_modules'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
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

      // Also React Compiler rules, and these two do sometimes point at a real
      // cascading render, so they stay visible as warnings.
      'react-hooks/set-state-in-effect': 'warn',
      'react-hooks/purity': 'warn',

      // tsc already reports unused locals and params; repeating it doubles output.
      '@typescript-eslint/no-unused-vars': 'off',
      // `any` appears where a third-party type is absent or wrong. Everything
      // around it is still checked.
      '@typescript-eslint/no-explicit-any': 'off',
    },
  },
);
