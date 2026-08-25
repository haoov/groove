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
