import type { ToolCheck } from '../../ipc/ipc';

// The forge axis: where code is hosted (github/gitlab). Not `provider`, which is where a task came from.

/** The reference sigil: `#` for GitHub, `!` for GitLab. */
export const mrSigil = (platform: string): string => (platform === 'github' ? '#' : '!');

/** An MR/PR reference as its forge writes it: `#42` / `!42`. */
export const mrRef = (platform: string, number: string | number): string =>
  `${mrSigil(platform)}${number}`;

/** The forge's product name, for copy like "Open in GitLab". */
export const forgeName = (platform: string): string =>
  platform === 'github' ? 'GitHub' : 'GitLab';

// ── Forge CLI readiness ───────────────────────────────────────────────────────

/** The forge CLIs the app shells out to. */
export const FORGE_CLIS: readonly string[] = ['glab', 'gh'];

/** How ready a forge CLI is, worst first. */
export type ForgeCliState = 'missing' | 'needs-auth' | 'needs-scope' | 'ready';

/** How ready a forge CLI is. `gh` needs the `project` scope; an unreadable scopes list is unknown, not empty. */
export function forgeCliState(tool: ToolCheck): ForgeCliState {
  if (!tool.path) return 'missing';
  if (tool.authed === false) return 'needs-auth';
  if (tool.name === 'gh' && tool.authed === true && tool.scopes && !tool.scopes.includes('project')) {
    return 'needs-scope';
  }
  return 'ready';
}
