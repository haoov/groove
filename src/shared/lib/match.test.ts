import { describe, expect, it } from 'vitest';
import { matchRanges, rankMatches } from './match';

const slugs = [
  'github.com/haoov/groove',
  'gitlab.example.com/wiremind/devops/mayonnaise',
  'gitlab.example.com/wiremind/devops/overwhelm',
  'gitlab.example.com/wiremind/data-science/pythie-cayzn-deploy',
  'gitlab.example.com/wiremind/cargo/cargostack-backend',
  'gitlab.example.com/security/keycloak-terraform',
];

const ranked = (q: string) => rankMatches(q, slugs, (s) => s).map((r) => r.item);

describe('rankMatches', () => {
  it('keeps every item for an empty query, in order', () => {
    expect(ranked('')).toEqual(slugs);
    expect(ranked('   ')).toEqual(slugs);
  });

  it('drops the scattered subsequences a bare matchRanges would keep', () => {
    // Every one of these contains m…a…y…o in order, so matchRanges accepts them.
    const loose = slugs.filter((s) => matchRanges('mayo', s) !== null);
    expect(loose.length).toBeGreaterThan(1);
    expect(ranked('mayo')).toEqual(['gitlab.example.com/wiremind/devops/mayonnaise']);
  });

  it('puts a contiguous match above a scattered one', () => {
    const out = ranked('car');
    expect(out[0]).toBe('gitlab.example.com/wiremind/cargo/cargostack-backend');
  });

  it('prefers a match at a word start', () => {
    const out = rankMatches('deploy', slugs, (s) => s);
    expect(out[0].item).toBe('gitlab.example.com/wiremind/data-science/pythie-cayzn-deploy');
  });

  it('returns nothing when nothing matches', () => {
    expect(ranked('zzzzz')).toEqual([]);
  });

  it('carries ranges that cover the query', () => {
    const [hit] = rankMatches('groove', slugs, (s) => s);
    const text = hit.item;
    const covered = hit.ranges.map(([a, b]) => text.slice(a, b)).join('');
    expect(covered.toLowerCase()).toBe('groove');
  });

  it('still allows a two-run subsequence', () => {
    // "devops" + "overwhelm" — one break, within the tightness budget.
    const out = rankMatches('devoverwhelm', slugs, (s) => s);
    expect(out.map((r) => r.item)).toContain('gitlab.example.com/wiremind/devops/overwhelm');
  });
});
