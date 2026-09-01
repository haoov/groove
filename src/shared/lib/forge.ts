import type { ToolCheck } from '../ipc/ipc';

// The forge axis: where CODE is hosted (github/gitlab) — not `provider`, which
// is where a TASK came from. One place for the sigil and the name; nine copies
// of `=== 'github' ? … : …` had made "not github" silently mean GitLab.

/** `#` for GitHub, `!` for GitLab — the reference sigil each forge uses. */
export const mrSigil = (platform: string): string => (platform === 'github' ? '#' : '!');

/** `#42` / `!42` — an MR/PR reference as its forge writes it. */
export const mrRef = (platform: string, number: string | number): string =>
  `${mrSigil(platform)}${number}`;

/** The forge's product name, for copy like "Open in GitLab". */
export const forgeName = (platform: string): string =>
  platform === 'github' ? 'GitHub' : 'GitLab';

// ── Forge CLI readiness ───────────────────────────────────────────────────────

/** The forge CLIs the app shells out to — `Git & forge`'s rows. */
export const FORGE_CLIS: readonly string[] = ['glab', 'gh'];

/** How ready a forge CLI is, worst first. */
export type ForgeCliState = 'missing' | 'needs-auth' | 'needs-scope' | 'ready';

/**
 * "Can this CLI do its job" — one rule for the first-run screen and the settings view.
 *
 * `gh` writes GitHub task properties, which needs the `project` scope. A scopes
 * list we could not read is UNKNOWN, not empty: a GH_TOKEN or a fine-grained PAT
 * prints none, and warning those users would be permanent.
 */
export function forgeCliState(tool: ToolCheck): ForgeCliState {
  if (!tool.path) return 'missing';
  if (tool.authed === false) return 'needs-auth';
  if (tool.name === 'gh' && tool.authed === true && tool.scopes && !tool.scopes.includes('project')) {
    return 'needs-scope';
  }
  return 'ready';
}
