import { defineConfig } from 'vitest/config';

// Unit tests only: pure logic, node environment.
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
