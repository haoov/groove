import { defineConfig } from 'vitest/config';

// Node by default: the pure-logic suite asserts node behaviour. A component test opts
// into jsdom with the `// @vitest-environment jsdom` docblock at the top of the file.
export default defineConfig({
  test: {
    include: ['src/**/*.test.{ts,tsx}'],
    environment: 'node',
  },
});
