/** Tokenize into alternating non-word / word segments (split preserves delimiters). */
function tokenize(s: string): string[] {
  return s.split(/(\w+)/);
}

/** DP LCS on token arrays; returns length table. */
function lcsTable(a: string[], b: string[]): number[][] {
  const m = a.length, n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 1; i <= m; i++)
    for (let j = 1; j <= n; j++)
      dp[i][j] = a[i - 1] === b[j - 1] ? dp[i - 1][j - 1] + 1 : Math.max(dp[i - 1][j], dp[i][j - 1]);
  return dp;
}

/** Backtrack LCS table → boolean mask: true = token is common (unchanged). */
function commonMask(dp: number[][], a: string[], b: string[]): { maskA: boolean[]; maskB: boolean[] } {
  const maskA = new Array(a.length).fill(false);
  const maskB = new Array(b.length).fill(false);
  let i = a.length, j = b.length;
  while (i > 0 && j > 0) {
    if (a[i - 1] === b[j - 1]) {
      maskA[i - 1] = true;
      maskB[j - 1] = true;
      i--; j--;
    } else if (dp[i - 1][j] >= dp[i][j - 1]) {
      i--;
    } else {
      j--;
    }
  }
  return { maskA, maskB };
}

/** Convert a boolean token mask to character ranges (exclusive end) in the original string. */
function changedRanges(tokens: string[], mask: boolean[]): [number, number][] {
  const ranges: [number, number][] = [];
  let offset = 0;
  let rangeStart = -1;
  for (let i = 0; i < tokens.length; i++) {
    const len = tokens[i].length;
    if (!mask[i]) {
      if (rangeStart === -1) rangeStart = offset;
    } else {
      if (rangeStart !== -1) {
        if (offset > rangeStart) ranges.push([rangeStart, offset]);
        rangeStart = -1;
      }
    }
    offset += len;
  }
  if (rangeStart !== -1 && offset > rangeStart) ranges.push([rangeStart, offset]);
  return ranges;
}

/**
 * Compute changed character ranges for a del/add line pair.
 * Returns char ranges (exclusive end) in `a` (del) and `b` (add) that differ.
 */
export function wordDiff(
  a: string, b: string,
): { delRanges: [number, number][]; addRanges: [number, number][] } {
  const ta = tokenize(a);
  const tb = tokenize(b);
  const dp = lcsTable(ta, tb);
  const { maskA, maskB } = commonMask(dp, ta, tb);
  return { delRanges: changedRanges(ta, maskA), addRanges: changedRanges(tb, maskB) };
}
