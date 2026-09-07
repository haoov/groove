// Test-only store control. Imported by tests, never by the app.

import { useStore } from './index';
import { newWorkspaceSession } from './session';
import type { AppState, SessionState } from './types';
import type { Repo, Task, Worktree } from '../ipc/ipc';

// Captured before any test runs; the slices' actions are stable closures.
const PRISTINE: AppState = { ...useStore.getState() };

/** Drops every session and every overlay flag. Call it in `beforeEach`. */
export function resetStore(): void {
  useStore.setState({ ...PRISTINE }, true);
}

export function testTask(over: Partial<Task> = {}): Task {
  return {
    short_id: 'plat-1',
    external_id: 'ext-1',
    provider: 'notion',
    external_url: null,
    title: 'Harden the CI gates',
    status: 'In progress',
    priority: null,
    last_synced_at: 0,
    ...over,
  };
}

export function testRepo(over: Partial<Repo> = {}): Repo {
  return {
    id: 'repo-1',
    host: 'gitlab.example.com',
    group_path: 'devops',
    project: 'mayo',
    local_path: '/w/main/gitlab.example.com/devops/mayo',
    ...over,
  };
}

export function testWorktree(over: Partial<Worktree> = {}): Worktree {
  return {
    id: 'wt-1',
    session_id: 'sess',
    repo_id: 'repo-1',
    branch: 'fix/parser',
    path: '/w/plat-1/mayo',
    base_ref: 'main',
    created_at: 0,
    ...over,
  };
}

/** Installs one focused workspace session and returns its id. */
export function seedSession(opts: {
  task?: Task | null;
  repos?: Repo[];
  worktrees?: Worktree[];
  patch?: Partial<SessionState>;
} = {}): string {
  const {
    task = testTask(),
    repos = [testRepo()],
    worktrees = [testWorktree()],
    patch,
  } = opts;
  const sess: SessionState = { ...newWorkspaceSession('task', task, worktrees, repos), ...patch };
  useStore.setState((s) => ({
    sessions: { ...s.sessions, [sess.id]: sess },
    sessionOrder: [...s.sessionOrder, sess.id],
    activeSessionId: sess.id,
    view: 'workspace',
  }));
  return sess.id;
}
