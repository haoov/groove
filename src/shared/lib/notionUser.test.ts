import { describe, expect, it } from 'vitest';
import { looksLikeNotionId } from './notionUser';

// A pasted id goes into the config as `notion.user_id`, and a wrong one there means
// "no tasks" with no error — Notion happily filters on an id that matches nobody.

describe('looksLikeNotionId', () => {
  it('takes both spellings Notion uses', () => {
    expect(looksLikeNotionId('227dcbda9c358221ad2201f823195954')).toBe(true);
    expect(looksLikeNotionId('6ef43008-8709-4c8a-9b96-4ca8c92e5f6f')).toBe(true);
    expect(looksLikeNotionId('  6EF43008-8709-4C8A-9B96-4CA8C92E5F6F  ')).toBe(true);
  });

  it('rejects near-misses', () => {
    expect(looksLikeNotionId('227dcbda9c358221ad2201f8231959')).toBe(false); // too short
    expect(looksLikeNotionId('zzzdcbda9c358221ad2201f823195954')).toBe(false); // not hex
    expect(looksLikeNotionId('')).toBe(false);
  });
});
