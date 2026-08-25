import { defineConfig } from 'vitest/config';

// Unit tests only: pure logic, colocated with its subject (shared/lib, shared/store,
// editor/cm, home).
// Nothing here touches the DOM or Tauri IPC, so no environment setup is needed —
// a component test would need jsdom and does not belong in this project yet.
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
